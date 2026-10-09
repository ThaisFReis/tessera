#![no_std]
//! # Tessera — voto secreto para qualquer governança, na Stellar.
//!
//! Três funções que uma governança já existente chama para realizar uma
//! votação em que
//!
//! - **sabe-se que uma pessoa votou**, público e auditável;
//! - **não se sabe em que ela votou**, e isso é secreto *para sempre*;
//! - **qualquer pessoa recalcula o resultado** e detecta uma mesa que minta.
//!
//! ## A ideia que organiza o desenho
//!
//! Num registro permanente, "criptografado hoje" significa "legível quando a
//! chave vazar". Então o ledger **nunca recebe um texto cifrado do voto**. Ele
//! recebe um compromisso de Pedersen `C = v·G + r·H`, que é perfeitamente
//! ocultante: para todo `v'` existe exatamente um `r'` com `v'·G + r'·H = C`,
//! logo `C` é uniforme em G1 e independente de `v`. Não há o que decifrar, com
//! qualquer poder computacional, para sempre.
//!
//! O vínculo é computacional, e a assimetria é deliberada. Se o log discreto
//! cair amanhã, uma mesa maliciosa passa a conseguir falsificar um total —
//! detectável, contestável, reparável. Se o sigilo fosse computacional, a queda
//! revelaria todos os votos já dados, retroativamente e sem reparo. Trocamos um
//! risco irreversível por um reversível.

// `env.events().publish()` está depreciado em favor da macro `#[contractevent]`,
// que muda o formato dos tópicos. **Esses eventos são interface congelada**
// (§5): `app/src/rede.ts` lista as votações lendo `topic[0] == "abrir"` e
// `topic[1]` como a proposta, e a janela útil do RPC é de 2.000 ledgers. Migrar
// sem migrar o leitor junto apaga a lista de votações do dapp. Virou T-012.
#![allow(deprecated)]
// A aridade das entradas é a ABI, e a ABI é interface congelada (§5): `abrir`
// tem 11 argumentos porque a proposta tem 11 campos. Reduzir exigiria um
// struct, logo mudar o ABI, logo redeploy.
#![allow(clippy::too_many_arguments)]

mod cripto;
mod tipos;

pub use tipos::{Erro, Pergunta, Proposta, ProvaCds, ProvaSoma};

use cripto::*;
use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{
    contract, contractimpl,
    crypto::bls12_381::{Bls12381Fr, Bls12381G1Affine, Bls12381G2Affine},
    symbol_short, Address, Bytes, BytesN, Env, Vec,
};
use tipos::{Chave, Instancia, MAX_OPCOES, MAX_PERGUNTAS, TAU};

#[contract]
pub struct Tessera;

#[contractimpl]
impl Tessera {
    // ===================== abrir =========================================

    /// Abre uma votação. Chamada pela governança integradora.
    ///
    /// `raiz_aptos` é a raiz de Merkle das folhas `H(0x00 ‖ addr_xdr ‖ peso)`
    /// da lista na data de corte. O contrato nunca vê a lista; confere caminhos.
    ///
    /// **Estende o TTL de `Proposta` e dos acumuladores ao teto da rede.** Sem
    /// isso eles arquivam em 7 dias (sonda 9) — o mesmo dia em que a janela do
    /// RPC fecha (sonda 8), e os dois níveis do verificador morreriam juntos.
    /// Custa ~0,6 XLM fixos, independente do comparecimento.
    ///
    /// ## A cédula
    ///
    /// `perguntas` é a cédula, em ordem. Uma cédula **confidencial** é aquela
    /// em que todas têm `confidencial: true`; uma **semiconfidencial** mistura.
    /// Não são dois caminhos de código — é o mesmo mecanismo em dois ajustes.
    pub fn abrir(
        env: Env,
        governanca: Address,
        proposta: BytesN<32>,
        perguntas: Vec<Pergunta>,
        raiz_aptos: BytesN<32>,
        mesa: Vec<Address>,
        limiar: u32,
        abre_em: u32,
        fecha_em: u32,
        anel: bool,
        secoes: u32,
        limite_secao: u32,
        rodada: u64,
        rodada_abertura: u64,
    ) -> Result<(), Erro> {
        governanca.require_auth();

        if env
            .storage()
            .persistent()
            .has(&Chave::Proposta(proposta.clone()))
        {
            return Err(Erro::PropostaJaExiste);
        }
        if perguntas.is_empty() || perguntas.len() > MAX_PERGUNTAS {
            return Err(Erro::PerguntasForaDaFaixa);
        }
        // O que limita a CPU é o total de opções **confidenciais**, porque é
        // nelas que mora a disjuntiva. As públicas são conferidas a olho.
        let mut conf_opcoes = 0u32;
        for q in perguntas.iter() {
            if q.opcoes < 2 || q.opcoes > MAX_OPCOES {
                return Err(Erro::OpcoesForaDaFaixa);
            }
            if q.confidencial {
                conf_opcoes += q.opcoes;
            }
        }
        if conf_opcoes > MAX_OPCOES {
            return Err(Erro::PerguntasForaDaFaixa);
        }
        // **Mesa vazia com limiar zero é a assembleia sem mesa nenhuma.**
        //
        // Antes o contrato a recusava, e a tela tinha de explicar uma exigência
        // sem função: numa cédula em anel a mesa não recebe parcela nenhuma,
        // então ela existia no estado e não servia para nada.
        //
        // Sem mesa ninguém endossa (`NaoEMembroDaMesa` recusa todo mundo), logo
        // ninguém reconstrói a abertura, logo nenhum total é publicado. O sigilo
        // é absoluto e o resultado é impossível — e isso é o desenho, não um
        // defeito. Ver DEC-003.
        //
        // O `!=` é um ou-exclusivo: limiar zero exige mesa vazia, e mesa vazia
        // exige limiar zero. Meio-termo não existe.
        if (limiar == 0) != mesa.is_empty() || limiar > mesa.len() {
            return Err(Erro::LimiarInvalido);
        }
        if fecha_em <= env.ledger().sequence() {
            return Err(Erro::PrazoNoPassado);
        }
        // A janela precisa existir. `abre_em` no passado é abertura imediata e
        // é legítimo — o que não pode é abrir depois de fechar, ou no mesmo
        // ledger, que daria uma votação de duração zero.
        if abre_em >= fecha_em {
            return Err(Erro::PrazoNoPassado);
        }
        // **A fechadura de tempo.** `rodada = 0` é proposta sem fechadura — a
        // da mesa, que continua existindo. Com fechadura, dois portões:
        //
        // 1. a rodada é **determinada** por `fim_tempo`, não escolhida. Sem
        //    isto, quem abre cifraria para uma rodada no passado, cuja
        //    assinatura já está publicada — e as cédulas abririam na hora de
        //    serem depositadas;
        // 2. o fechamento está no futuro. O relógio é do ledger, não de quem
        //    abre.
        //
        // A fórmula vem do `core`, o mesmo módulo que o cliente usa para
        // cifrar. Se divergissem, a cédula seria cifrada para uma rodada e
        // destrancada por outra — e o jeito de não divergir é não ter duas.
        // **As duas rodadas da baliza, e as duas têm de estar no futuro.**
        //
        // No futuro é o que importa, e é a mesma razão nas duas: uma rodada já
        // vencida tem assinatura publicada. Para a fechadura, isso faria as
        // cédulas abrirem na hora de serem depositadas; para a seção, deixaria
        // quem organiza moer até isolar alguém (DEC-012). A rodada **é** o
        // prazo: o instante dela é `instante(rodada)`, então não há campo de
        // tempo separado para divergir.
        let agora = env.ledger().timestamp();
        if rodada != 0 && instante_da_rodada(rodada) <= agora {
            return Err(Erro::RodadaNaoFecha);
        }
        if rodada_abertura != 0 {
            if instante_da_rodada(rodada_abertura) <= agora {
                return Err(Erro::RodadaNaoFecha);
            }
            // E o comparecimento abre antes de a urna fechar, senão ninguém
            // chega a votar.
            if rodada != 0 && rodada_abertura >= rodada {
                return Err(Erro::PrazoNoPassado);
            }
        }
        // **Seções numa votação fechada exigem a baliza.** Sem ela não há nada
        // contra o que conferir a seção, e quem comparece escolheria a sua — que
        // é precisamente o ataque da DEC-012 pela outra porta. Uma seção só
        // continua dispensando a baliza, porque não há escolha a fazer.
        if anel && !e_aberta(&raiz_aptos) && secoes > 1 && rodada_abertura == 0 {
            return Err(Erro::SecaoInvalida);
        }
        // Membros repetidos reduziriam o limiar efetivo sem que aparecesse.
        for (i, m) in mesa.iter().enumerate() {
            if mesa.iter().take(i).any(|o| o == m) {
                return Err(Erro::MembroRepetido);
            }
        }

        // H é caro (2.653.011) e constante: calcula uma vez na instância.
        //
        // **O TTL da instância NÃO é estendido aqui, de propósito.** Estender
        // instância e código ao teto custa ~181 XLM, porque o Wasm tem 21 KB e
        // o aluguel é proporcional ao tamanho. Fazer isso dentro de `abrir()`
        // faria a primeira governança a abrir uma proposta pagar 100× o que
        // todas as outras pagam — medido na testnet: 182,39 XLM na primeira
        // chamada, 1,72 XLM na segunda. Quem mantém o contrato vivo é
        // `manter()`, que qualquer pessoa chama.
        if !env.storage().instance().has(&Instancia::GeradorH) {
            let h = calcular_h(&env);
            env.storage().instance().set(&Instancia::GeradorH, &h);
        }

        if secoes == 0 || (!anel && secoes != 1) {
            return Err(Erro::SecaoInvalida);
        }
        // O limite só governa a votação aberta, onde ninguém sabe quem vem.
        // Na fechada a lista é conhecida e a divisão sai dela na abertura.
        let aberta = e_aberta(&raiz_aptos);
        if limite_secao > 0 && (!aberta || secoes != 1) {
            return Err(Erro::SecaoInvalida);
        }
        let n_perguntas = perguntas.len();
        let p = Proposta {
            perguntas,
            raiz_aptos,
            mesa,
            limiar,
            abre_em,
            fecha_em,
            anel,
            secoes,
            limite_secao,
            rodada,
            rodada_abertura,
        };
        if anel {
            // Um anel por seção, e cada um nasce vazio: sem esta escrita, o
            // primeiro `comparecer` daquela seção não teria onde se somar.
            let kn = Chave::Caderno(proposta.clone());
            env.storage().persistent().set(&kn, &0u32);
            guardar_longo(&env, &kn);
            for s in 0..secoes {
                let k = Chave::Anel(proposta.clone(), s);
                env.storage()
                    .persistent()
                    .set(&k, &Vec::<Bls12381G1Affine>::new(&env));
                guardar_longo(&env, &k);
            }
        }
        env.storage()
            .persistent()
            .set(&Chave::Proposta(proposta.clone()), &p);
        guardar_longo(&env, &Chave::Proposta(proposta.clone()));

        // Acumuladores começam no infinito: o compromisso de ninguém. Só as
        // perguntas sigilosas têm acumulador; **todas** têm total público,
        // porque quem abre o voto por `votar_publico()` responde a cédula
        // inteira em claro, inclusive as perguntas sigilosas.
        let zero = infinito(&env);
        for (q, pg) in p.perguntas.iter().enumerate() {
            let q = q as u32;
            for j in 0..pg.opcoes {
                if pg.confidencial {
                    let k = Chave::Acum(proposta.clone(), q, j);
                    env.storage().persistent().set(&k, &zero);
                    guardar_longo(&env, &k);
                }
                let kp = Chave::TotalPublico(proposta.clone(), q, j);
                env.storage().persistent().set(&kp, &0u32);
                guardar_longo(&env, &kp);
            }
        }
        let kc = Chave::Comparecimento(proposta.clone());
        env.storage().persistent().set(&kc, &(0u32, 0u32));
        guardar_longo(&env, &kc);

        env.events().publish(
            (symbol_short!("abrir"), proposta),
            (
                n_perguntas,
                conf_opcoes,
                abre_em,
                fecha_em,
                limiar,
                anel,
                secoes,
            ),
        );
        Ok(())
    }

    // ===================== votar =========================================

    /// Registra uma cédula em sigilo. Quem vota paga a taxa.
    ///
    /// Nas perguntas sigilosas entra `C_j = v_j·G + r_j·H` por opção, mais as
    /// provas de boa formação. O `v_j` não entra, não transita, e é destruído
    /// no cliente. Nas perguntas públicas a resposta vai em claro, na mesma
    /// transação — é a cédula semiconfidencial.
    ///
    /// `compromissos` e `provas` vêm achatados sobre as perguntas **sigilosas**
    /// em ordem; `escolhas` sobre as **públicas**. Uma prova de soma por
    /// pergunta sigilosa, em `provas_soma`.
    ///
    /// ## Por que uma prova de soma por pergunta
    ///
    /// Não é otimização, é correção. Uma prova única sobre todos os
    /// compromissos da cédula afirmaria `Σ(tudo) = peso` — e com `peso = 1`
    /// isso obriga o eleitor a marcar **exatamente uma opção na cédula
    /// inteira**. Quem responde a pergunta 1 seria forçado a abster-se das
    /// outras. Cada pergunta precisa do seu `Σ(suas opções) = peso`.
    ///
    /// Custo medido: 9.805.000 fixos + 13.501.500 por opção confidencial.
    /// Três perguntas de 2 opções sigilosas custam 22,7% do teto — menos que
    /// as mesmas três como propostas separadas, porque a prova de aptidão por
    /// Merkle é paga **uma vez**.
    pub fn votar(
        env: Env,
        proposta: BytesN<32>,
        votante: Address,
        compromissos: Vec<Bls12381G1Affine>,
        provas: Vec<ProvaCds>,
        provas_soma: Vec<ProvaSoma>,
        escolhas: Vec<u32>,
        caminho: Vec<BytesN<32>>,
        indice: u32,
        peso: u32,
    ) -> Result<(), Erro> {
        votante.require_auth();
        let p = abrir_proposta(&env, &proposta)?;
        // Os dois modos não se misturam: numa proposta de anel, votar pelo
        // endereço recriaria o vínculo que o anel existe para quebrar.
        if p.anel {
            return Err(Erro::ModoErrado);
        }

        if env.ledger().sequence() < p.abre_em {
            return Err(Erro::VotacaoAindaNaoComecou);
        }
        if env.ledger().sequence() >= p.fecha_em {
            return Err(Erro::VotacaoEncerrada);
        }
        let (n_conf, n_publ, n_perg_conf) = formato(&p);
        if compromissos.len() != n_conf
            || provas.len() != n_conf
            || provas_soma.len() != n_perg_conf
            || escolhas.len() != n_publ
        {
            return Err(Erro::ArgumentoMalFormado);
        }
        // Sem anel não há seção: a folha é a da seção zero.
        conferir_aptidao(&env, &p, &votante, peso, indice, &caminho)?;
        marcar_votou(&env, &proposta, &votante)?;

        conferir_e_somar(
            &env,
            &p,
            &proposta,
            &votante.clone().to_xdr(&env),
            &compromissos,
            &provas,
            &provas_soma,
            &escolhas,
            peso,
        )?;

        // O evento é o que o nível 2 do verificador lê dos arquivos de
        // histórico para recalcular o acumulador sem depender do RPC (§8.3).
        env.events().publish(
            (symbol_short!("votar"), proposta, votante),
            (compromissos, escolhas),
        );
        Ok(())
    }

    /// **O caderno.** Identificado, público, e separado da urna.
    ///
    /// A pessoa prova que está na lista de aptos e registra a chave pública com
    /// que vai assinar o anel. É o único ato em que o nome dela aparece — e é o
    /// que permite voto obrigatório: `aptos − compareceram` é a lista de quem
    /// faltou.
    ///
    /// Só vale **antes** de `abre_em`. Depois que a votação abre, o anel está
    /// congelado: aceitar mais um membro mudaria o conjunto debaixo de quem já
    /// votou, e uma assinatura feita sobre o conjunto antigo deixaria de fechar.
    ///
    /// Devolve o tamanho do anel até agora, que é o tamanho do esconderijo.
    pub fn comparecer(
        env: Env,
        proposta: BytesN<32>,
        votante: Address,
        chave_anel: Bls12381G1Affine,
        caminho: Vec<BytesN<32>>,
        indice: u32,
        secao: u32,
    ) -> Result<u32, Erro> {
        votante.require_auth();
        let p = abrir_proposta(&env, &proposta)?;
        if !p.anel {
            return Err(Erro::ModoErrado);
        }
        if env.ledger().sequence() >= p.abre_em {
            return Err(Erro::ComparecimentoEncerrado);
        }

        let kc = Chave::Compareceu(proposta.clone(), votante.clone());
        if env.storage().persistent().has(&kc) {
            return Err(Erro::JaCompareceu);
        }

        // Na aberta, quem decide a seção é o contrato — por
        // `H(proposta ‖ endereço) mod secoes`.
        //
        // **Ordem de chegada não serve, e descobrir isso custou uma rodada.** A
        // seção nomeia a entrada `Anel(proposta, secao)` que a transação vai
        // escrever, e o footprint é declarado na *simulação*. Se a seção só
        // existisse na *aplicação*, cada comparecimento declararia uma entrada e
        // escreveria outra — medido na testnet: uma por ledger, e duas de nove
        // recusadas com `txFailed`.
        //
        // Derivar do endereço resolve porque o cliente calcula o mesmo antes de
        // enviar. O preço é que dá para moer endereços até cair numa seção
        // escolhida; numa votação aberta isso não tira nada de ninguém, já que
        // escolher o próprio esconderijo não encolhe o de outra pessoa — e o
        // piso de `TAU` continua valendo.
        let kn = Chave::Caderno(proposta.clone());
        let caderno: u32 = env.storage().persistent().get(&kn).unwrap_or(0);

        let secao = if e_aberta(&p.raiz_aptos) {
            if !caminho.is_empty() {
                return Err(Erro::VotacaoAberta);
            }
            // **Enche e abre a próxima.** A seção vem da ordem de chegada, que
            // é o que deixa o organizador parar de adivinhar quanta gente vem.
            //
            // Isso exige do cliente uma coisa que não é óbvia: a seção só existe
            // na *aplicação*, mas o footprint — que nomeia `Anel(proposta, s)` —
            // é declarado na *simulação*. Quem manda uma cédula precisa declarar
            // uma **janela** de seções, não só a prevista, senão uma rajada de
            // gente faz cada transação escrever onde não declarou. Medido: sem a
            // janela, uma por ledger e recusas com `txFailed`.
            caderno.checked_div(p.limite_secao).unwrap_or(0)
        } else {
            if secao >= p.secoes {
                return Err(Erro::SecaoInvalida);
            }
            conferir_aptidao(&env, &p, &votante, 1, indice, &caminho)?;
            // **A seção vem da baliza, e o contrato confere** (DEC-012). Antes
            // ela vinha presa na folha de Merkle, e quem montava a lista a
            // escolhia; agora sai de uma assinatura que não existia quando a
            // proposta foi aberta. Aceitar a seção que a chamada afirma sem
            // conferir devolveria a escolha para quem vota, o que é o mesmo
            // ataque pela outra porta.
            if p.rodada_abertura != 0 && p.secoes > 1 {
                let assinatura: BytesN<96> = env
                    .storage()
                    .persistent()
                    .get(&Chave::Abertura(proposta.clone()))
                    .ok_or(Erro::BalizaNaoRegistrada)?;
                if secao != secao_da_baliza(&env, &assinatura, &votante, p.secoes) {
                    return Err(Erro::SecaoInvalida);
                }
            }
            secao
        };
        // Validado aqui, uma vez. Depois disso o digesto do conjunto prende
        // estes pontos exatos, então a cédula não precisa revalidar os `n`.
        validar(&env, &chave_anel)?;

        // Com split automático a seção nova não existia na abertura: ela nasce
        // vazia com quem chega primeiro nela.
        let ka = Chave::Anel(proposta.clone(), secao);
        let mut anel: Vec<Bls12381G1Affine> = env
            .storage()
            .persistent()
            .get(&ka)
            .unwrap_or_else(|| Vec::new(&env));
        anel.push_back(chave_anel);
        let tamanho = anel.len();
        env.storage().persistent().set(&ka, &anel);
        guardar_longo(&env, &ka);

        env.storage().persistent().set(&kc, &secao);
        env.storage().persistent().set(&kn, &(caderno + 1));
        guardar_longo(&env, &kn);
        guardar_longo(&env, &kc);

        env.events().publish(
            (symbol_short!("comparec"), proposta, votante),
            (secao, tamanho),
        );
        Ok(tamanho)
    }

    /// **A urna.** Sem endereço de membro, e sem nada que leve a um.
    ///
    /// Quem assina a transação é uma chave efêmera que só paga a taxa. A
    /// elegibilidade vem da assinatura em anel: ela prova que quem montou esta
    /// cédula conhece a chave de **um** dos que compareceram, e nada diz qual.
    ///
    /// O voto duplo é barrado pela imagem de chave, não pelo endereço: a mesma
    /// pessoa produz sempre a mesma `I = x·Hp` dentro desta proposta, e o
    /// contrato recusa a segunda — **sem saber de quem é**.
    #[allow(clippy::too_many_arguments)]
    pub fn votar_anonimo(
        env: Env,
        proposta: BytesN<32>,
        secao: u32,
        anel: Vec<Bls12381G1Affine>,
        imagem: Bls12381G1Affine,
        c0: Bls12381Fr,
        z: Vec<Bls12381Fr>,
        compromissos: Vec<Bls12381G1Affine>,
        provas: Vec<ProvaCds>,
        provas_soma: Vec<ProvaSoma>,
        escolhas: Vec<u32>,
        cripto: Bytes,
    ) -> Result<(), Erro> {
        let p = abrir_proposta(&env, &proposta)?;
        if !p.anel {
            return Err(Erro::ModoErrado);
        }
        if secao >= quantas_secoes(&env, &proposta, &p) {
            return Err(Erro::SecaoInvalida);
        }
        if env.ledger().sequence() < p.abre_em {
            return Err(Erro::VotacaoAindaNaoComecou);
        }
        if env.ledger().sequence() >= p.fecha_em {
            return Err(Erro::VotacaoEncerrada);
        }
        let (n_conf, n_publ, n_perg_conf) = formato(&p);
        if compromissos.len() != n_conf
            || provas.len() != n_conf
            || provas_soma.len() != n_perg_conf
            || escolhas.len() != n_publ
            || anel.is_empty()
            || z.len() != anel.len()
        {
            return Err(Erro::ArgumentoMalFormado);
        }
        // Um criptograma de 160 bytes por opção confidencial (SPEC §5), ou
        // nenhum quando a proposta não tem fechadura. O contrato não os lê — ele
        // **não pode**: o host só expõe `pairing_check`, sem pareamento com
        // saída de valor, e decifrar precisa do valor. Ele só confere a forma e
        // os carrega no evento, e quem decifra é qualquer pessoa, depois da
        // rodada. Quem recusa um total que mente é o compromisso de Pedersen.
        let esperado = if p.rodada == 0 {
            0
        } else {
            TAMANHO_CRIPTOGRAMA * n_conf
        };
        if cripto.len() != esperado {
            return Err(Erro::CriptogramaMalFormado);
        }

        // O digesto do conjunto é calculado uma vez, na primeira cédula, e
        // guardado. Daí em diante conferir o anel é comparar 32 bytes em vez de
        // reler `n` pontos do estado.
        let kd = Chave::DigestoAnel(proposta.clone(), secao);
        let registrado: BytesN<32> = match env.storage().persistent().get(&kd) {
            Some(d) => d,
            None => {
                let congelado: Vec<Bls12381G1Affine> = env
                    .storage()
                    .persistent()
                    .get(&Chave::Anel(proposta.clone(), secao))
                    .ok_or(Erro::ModoErrado)?;
                let d = digesto_anel(&env, &congelado);
                env.storage().persistent().set(&kd, &d);
                guardar_longo(&env, &kd);
                d
            }
        };
        if digesto_anel(&env, &anel) != registrado {
            return Err(Erro::AnelInvalido);
        }
        // Numa votação sem seções, o anel é quem apareceu, e isso é problema de
        // quem organizou — a tela avisa. Com seções, **alguém decidiu** quem se
        // esconde atrás de quem, e uma seção minúscula entregaria o voto de
        // quem caiu nela. Aqui a recusa vale mais que o aviso: o mesmo princípio
        // do PROTOCOLO §6.6, que prefere falhar a vazar.
        if quantas_secoes(&env, &proposta, &p) > 1 && anel.len() < TAU {
            return Err(Erro::AnonimatoInsuficiente);
        }

        validar(&env, &imagem)?;
        let msg = mensagem_cedula(&env, &proposta, &compromissos, &escolhas);
        let pre = compor_anel(&env, &msg, &registrado);
        let hp = calcular_hp(&env, &proposta);
        if !verificar_anel(&env, &pre, &gerador_g(&env), &hp, &anel, &imagem, &c0, &z) {
            return Err(Erro::AnelInvalido);
        }

        // A imagem entra como chave pelo seu hash: 32 bytes em vez de 96, e o
        // que importa é a colisão, não o ponto.
        let ki = Chave::ImagemUsada(
            proposta.clone(),
            env.crypto()
                .sha256(&Bytes::from_array(&env, &imagem.to_array()))
                .into(),
        );
        if env.storage().persistent().has(&ki) {
            return Err(Erro::ImagemJaUsada);
        }

        // A identidade que prende as provas é a imagem. Ela é única por pessoa
        // e por proposta, e não diz quem é — exatamente o que o contexto
        // precisa ser.
        let ident = Bytes::from_array(&env, &imagem.to_array());
        conferir_e_somar(
            &env,
            &p,
            &proposta,
            &ident,
            &compromissos,
            &provas,
            &provas_soma,
            &escolhas,
            1,
        )?;

        env.storage().persistent().set(&ki, &true);
        guardar_longo(&env, &ki);

        // **A cadeia da seção.** Trinta e seis bytes que não crescem, e que
        // prendem a lista de compromissos daquela seção à ordem em que as
        // cédulas chegaram. Na apuração, re-encadear a lista apresentada é o
        // que recusa omissão e invenção (INV-20) — e sem isso quem apura
        // escolheria quais cédulas contar.
        if p.rodada != 0 {
            let kc = Chave::Cadeia(proposta.clone(), secao);
            let (anterior, quantas) = cadeia_de(&env, &proposta, secao);
            let mut buf = Bytes::from_array(&env, &anterior.to_array());
            for c in compromissos.iter() {
                buf.extend_from_array(&c.to_array());
            }
            let novo: BytesN<32> = env.crypto().sha256(&buf).into();
            env.storage().persistent().set(&kc, &(novo, quantas + 1));
            guardar_longo(&env, &kc);
        }

        // O evento NÃO carrega remetente. É o que separa este evento do
        // `votar`: ali o tópico tem o endereço, aqui tem a imagem.
        //
        // E carrega o criptograma, que é o que torna a apuração possível sem
        // mesa: evento não toca entrada compartilhada, então não tem o problema
        // de rajada da §11-D, e a retenção do RPC público (7 dias, medida) é
        // maior que qualquer janela de votação.
        env.events().publish(
            (symbol_short!("anonimo"), proposta),
            (imagem, compromissos, escolhas, cripto),
        );
        Ok(())
    }

    // `votar_publico` existia aqui e foi **removido**.
    //
    // Ele deixava qualquer eleitor abrir a própria cédula inteira no ledger,
    // inclusive as perguntas sigilosas. Era revelação voluntária, e parecia um
    // direito — mas um voto aberto on-chain é a forma mais forte de coação que
    // existe: quem coage confere sozinho, lendo o ledger, sem precisar da
    // pessoa na frente. Coação verificável em escala.
    //
    // Pior, em bloco ele particionava o eleitorado e derrubava a apuração por
    // `TAU` (§6.6). Três pessoas abrindo o voto vetavam a assembleia inteira —
    // negação de serviço contra a própria eleição.
    //
    // Nenhuma eleição séria aceita isso. A prática é proteger a célula pequena
    // **antes**, agregando seções abaixo de um piso, e nunca recusar a contagem
    // depois. Sem revelação individual não
    // há partição a induzir, e `TAU` volta a ser o que deveria: quórum,
    // declarado na abertura.
    //
    // **Pergunta pública continua existindo** e é outra coisa: `Pergunta {
    // confidencial: false }` vale para todo mundo, é decidida pelo estatuto em
    // `abrir()`, e vai em claro no `escolhas` do `votar()` normal. O que saiu
    // foi o indivíduo abrindo o que o estatuto mandou manter em sigilo.

    // ===================== apurar ========================================

    /// Apura — **um endosso por vez, uma transação por membro.**
    ///
    /// A mesa publica `(T_j, R_j)` e o contrato **confere** `A_j == T_j·G +
    /// R_j·H`. Não procura o total: a sonda 5 mostrou que procurar por força
    /// bruta custa 124.277 por unidade de peso e tem teto de ~3.218, enquanto
    /// conferir custa 5.408.931 **constante no comparecimento** — medido
    /// idêntico para 250 e para 1.000.000. Quem procura é a mesa, fora da
    /// cadeia, onde procurar é de graça.
    ///
    /// Revelar `R_j = Σ r_{i,j}` não revela nenhum `r_{i,j}`: é a soma de `n`
    /// valores uniformes, e conhecer a soma de `n` incógnitas não determina
    /// nenhuma delas. E `R_j` chega à mesa por shares de Shamir, então nenhum
    /// `r` individual se junta em lugar algum (PROTOCOLO §4.3).
    ///
    /// ## Por que endosso e não co-assinatura
    ///
    /// A primeira versão pedia `k` autorizações numa transação só. Isso não é
    /// expressável pelo ferramental da Stellar — `stellar tx sign` assina o
    /// envelope, não as entradas de autorização do Soroban, e a rede recusa
    /// com `TxBadAuthExtra`.
    ///
    /// E a correção é melhor que o desenho original: os `k` membros são `k`
    /// pessoas em `k` máquinas, que não têm como passar um envelope de mão em
    /// mão com conforto. Cada uma manda **a sua** transação, e o contrato
    /// conta. Além disso cada endosso fica preso ao digest de `(totais,
    /// aberturas)`, então ninguém endossa "a apuração" em abstrato: endossa
    /// **estes números**, e discordar é não endossar.
    ///
    /// Toda chamada confere tudo. Um endosso de números falsos é recusado
    /// na hora, no primeiro membro que tentar — não no último.
    ///
    /// Devolve o resultado quando o `k`-ésimo endosso fecha, e `None` antes.
    pub fn apurar(
        env: Env,
        proposta: BytesN<32>,
        membro: Address,
        totais: Vec<u32>,
        aberturas: Vec<Bls12381Fr>,
    ) -> Result<Option<Vec<u32>>, Erro> {
        membro.require_auth();
        let p = abrir_proposta(&env, &proposta)?;

        if env.ledger().sequence() < p.fecha_em {
            return Err(Erro::VotacaoAindaAberta);
        }
        if env
            .storage()
            .persistent()
            .has(&Chave::Resultado(proposta.clone()))
        {
            return Err(Erro::JaApurada);
        }
        // `totais` e `aberturas` cobrem só as opções **confidenciais**, em
        // ordem de pergunta. As públicas já estão somadas em claro.
        let (n_conf, _n_publ, _n_perg_conf) = formato(&p);
        if totais.len() != n_conf || aberturas.len() != n_conf {
            return Err(Erro::ArgumentoMalFormado);
        }
        if !p.mesa.iter().any(|m| m == membro) {
            return Err(Erro::NaoEMembroDaMesa);
        }

        // **A regra de τ.** Ver PROTOCOLO §6.6. Ou ninguém votou em sigilo — e não
        // há sigilo a proteger — ou pelo menos τ votaram. Uma coligação que
        // publique os próprios votos de propósito encolheria o conjunto
        // secreto até determiná-lo; aqui ela trava a apuração em vez de ler os
        // votos. A troca é deliberada: uma votação travada é contestável e
        // repetível, um voto vazado não volta atrás.
        let (conf, _publ): (u32, u32) = env
            .storage()
            .persistent()
            .get(&Chave::Comparecimento(proposta.clone()))
            .unwrap();
        if conf > 0 && conf < TAU {
            return Err(Erro::AnonimatoInsuficiente);
        }

        let g = gerador_g(&env);
        let h: Bls12381G1Affine = env
            .storage()
            .instance()
            .get(&Instancia::GeradorH)
            .ok_or(Erro::PropostaNaoExiste)?;
        let bls = env.crypto().bls12_381();

        let mut resultado = Vec::new(&env);
        let mut off = 0u32;
        for (q, pg) in p.perguntas.iter().enumerate() {
            let q = q as u32;

            // Como a v1 é um-voto-por-pessoa, a soma dos totais confidenciais
            // **de cada pergunta** tem de ser exatamente o número de cédulas
            // em sigilo. Pega uma mesa que invente votos sem nem precisar
            // abrir o acumulador — e agora pega também quem tente compensar
            // uma pergunta com outra.
            if pg.confidencial {
                let mut soma_t = 0u32;
                for j in 0..pg.opcoes {
                    soma_t += totais.get(off + j).unwrap();
                }
                if soma_t != conf {
                    return Err(Erro::TotalDiferenteDoComparecimento);
                }
            }

            for j in 0..pg.opcoes {
                let publico: u32 = env
                    .storage()
                    .persistent()
                    .get(&Chave::TotalPublico(proposta.clone(), q, j))
                    .unwrap();

                if !pg.confidencial {
                    resultado.push_back(publico);
                    continue;
                }

                let a: Bls12381G1Affine = env
                    .storage()
                    .persistent()
                    .get(&Chave::Acum(proposta.clone(), q, j))
                    .unwrap();

                // A_j == T_j·G + R_j·H, um MSM de 2 termos. 5.408.931 medidos.
                let mut ps = Vec::new(&env);
                let mut ss = Vec::new(&env);
                ps.push_back(g.clone());
                ps.push_back(h.clone());
                ss.push_back(fr(&env, totais.get(off + j).unwrap()));
                ss.push_back(aberturas.get(off + j).unwrap());
                if bls.g1_msm(ps, ss) != a {
                    return Err(Erro::AberturaNaoFecha);
                }

                resultado.push_back(totais.get(off + j).unwrap() + publico);
            }
            if pg.confidencial {
                off += pg.opcoes;
            }
        }

        // Daqui para baixo os números conferem. O que falta é quórum.
        let digest = digest_da_apuracao(&env, &totais, &aberturas);
        let meu = Chave::Endosso(proposta.clone(), digest.clone(), membro.clone());
        if env.storage().persistent().has(&meu) {
            return Err(Erro::MembroJaEndossou);
        }
        env.storage().persistent().set(&meu, &true);

        let conta = Chave::Endossos(proposta.clone(), digest.clone());
        let quantos: u32 = env.storage().persistent().get(&conta).unwrap_or(0) + 1;
        env.storage().persistent().set(&conta, &quantos);
        guardar_longo(&env, &conta);

        env.events().publish(
            (symbol_short!("endosso"), proposta.clone(), membro),
            (digest.clone(), quantos, p.limiar),
        );

        if quantos < p.limiar {
            return Ok(None);
        }

        let k = Chave::Resultado(proposta.clone());
        env.storage().persistent().set(&k, &resultado);
        guardar_longo(&env, &k);

        env.events().publish(
            (symbol_short!("apurar"), proposta),
            (totais, aberturas, resultado.clone()),
        );
        Ok(Some(resultado))
    }

    /// **A apuração sem mesa.** Qualquer pessoa, depois que a janela fechou e a
    /// rodada da baliza venceu.
    ///
    /// Quem apura apresenta a lista ordenada de **todos** os compromissos da
    /// seção, um `abertas[i]` por cédula dizendo quais o criptograma abriu, e
    /// `(totais, aberturas)` sobre as abertas. O contrato re-encadeia a lista
    /// (INV-20), soma os compromissos marcados e confere o mesmo MSM de dois
    /// termos de sempre — a solidez não muda, porque a prova CDS já garantiu
    /// `v ∈ {0,1}` em cada compromisso na hora do voto, então o compromisso é
    /// vinculante e a equação sobre o subconjunto força `T = Σvᵢ` e `R = Σrᵢ`.
    ///
    /// E guarda só se abrir **mais** cédulas que a apuração já guardada
    /// (INV-21). É isso que impede travar: quem omitir uma cédula honesta é
    /// sobreposto por qualquer pessoa que a inclua — e qualquer pessoa consegue,
    /// porque a chave da rodada é pública. Basta um observador honesto, não que
    /// os votantes voltem.
    ///
    /// **Não há portão de `τ` sobre quantas abriram, e a ausência é
    /// deliberada** (INV-25, DEC-011). O piso de `τ` protege o conjunto de
    /// anonimato, que é o anel, e `votar_anonimo` já o impõe lá. Exigi-lo aqui
    /// sobre o subconjunto aberto daria a qualquer um o poder de travar a
    /// apuração sabotando o próprio criptograma — a falha de liveness induzível
    /// de fora que a remoção de `votar_publico` havia fechado.
    pub fn apurar_secao(
        env: Env,
        proposta: BytesN<32>,
        quem: Address,
        secao: u32,
        compromissos: Vec<Bls12381G1Affine>,
        abertas: Vec<bool>,
        totais: Vec<u32>,
        aberturas: Vec<Bls12381Fr>,
    ) -> Result<u32, Erro> {
        quem.require_auth();
        let p = abrir_proposta(&env, &proposta)?;
        if !p.anel || p.rodada == 0 {
            return Err(Erro::ModoErrado);
        }
        if secao >= quantas_secoes(&env, &proposta, &p) {
            return Err(Erro::SecaoInvalida);
        }
        // **Os dois portões, e os dois são o relógio do ledger.** A sequência
        // fechou a janela; a rodada é o instante em que a chave que decifra as
        // cédulas passa a existir no mundo. Antes dos dois, nenhum placar — nem
        // parcial — sai daqui (INV-18), e isso não depende de suposição nenhuma
        // sobre a baliza.
        if env.ledger().sequence() < p.fecha_em {
            return Err(Erro::VotacaoAindaAberta);
        }
        if env.ledger().timestamp() < instante_da_rodada(p.rodada) {
            return Err(Erro::RelogioAindaNaoAbriu);
        }

        let (n_conf, _, _) = formato(&p);
        let (cadeia, quantas) = cadeia_de(&env, &proposta, secao);
        if abertas.len() != quantas
            || compromissos.len() != quantas * n_conf
            || totais.len() != n_conf
            || aberturas.len() != n_conf
        {
            return Err(Erro::ArgumentoMalFormado);
        }

        // **Re-encadear é o que recusa omissão e invenção** (INV-20). A cadeia
        // de 32 bytes que as cédulas escreveram prende a lista inteira, na
        // ordem: tirar uma cédula muda o encadeamento, trocar um compromisso
        // também. Sem isto, quem apura escolheria quais cédulas contar.
        let mut acc = BytesN::from_array(&env, &[0u8; 32]);
        for i in 0..quantas {
            let mut buf = Bytes::from_array(&env, &acc.to_array());
            for j in 0..n_conf {
                buf.extend_from_array(&compromissos.get(i * n_conf + j).unwrap().to_array());
            }
            acc = env.crypto().sha256(&buf).into();
        }
        if acc != cadeia {
            return Err(Erro::CadeiaNaoFecha);
        }

        // **Monotonicidade** (INV-21): só um conjunto estritamente maior
        // substitui o guardado. É a peça que impede travar — quem omitir uma
        // cédula honesta é sobreposto por qualquer pessoa que a inclua.
        let mut quantas_abertas = 0u32;
        for b in abertas.iter() {
            if b {
                quantas_abertas += 1;
            }
        }
        let kr = Chave::ResultadoSecao(proposta.clone(), secao);
        let antes: Option<(u32, Vec<u32>)> = env.storage().persistent().get(&kr);
        if let Some((ja, _)) = &antes {
            if quantas_abertas <= *ja {
                return Err(Erro::NaoMelhora);
            }
        }

        // E a conferência de sempre, agora sobre o subconjunto: a soma dos
        // compromissos marcados tem de abrir em `T_j·G + R_j·H`. A solidez não
        // muda, porque a prova CDS já garantiu `v ∈ {0,1}` em cada compromisso
        // na hora do voto — o compromisso é vinculante, e a equação sobre o
        // subconjunto força `T_j = Σvᵢ` e `R_j = Σrᵢ`.
        //
        // Somar `n` pontos e fazer **um** MSM de dois termos é o caminho
        // barato: somas em G1 custam uma fração de uma multiplicação, e
        // conferir cédula por cédula pediria uma multiplicação cada.
        let g = gerador_g(&env);
        let h: Bls12381G1Affine = env
            .storage()
            .instance()
            .get(&Instancia::GeradorH)
            .ok_or(Erro::PropostaNaoExiste)?;
        let bls = env.crypto().bls12_381();

        let mut placar = Vec::new(&env);
        for j in 0..n_conf {
            let mut soma = infinito(&env);
            for i in 0..quantas {
                if abertas.get(i).unwrap() {
                    soma = bls.g1_add(&soma, &compromissos.get(i * n_conf + j).unwrap());
                }
            }
            let mut ps = Vec::new(&env);
            let mut ss = Vec::new(&env);
            ps.push_back(g.clone());
            ps.push_back(h.clone());
            ss.push_back(fr(&env, totais.get(j).unwrap()));
            ss.push_back(aberturas.get(j).unwrap());
            if bls.g1_msm(ps, ss) != soma {
                return Err(Erro::AberturaNaoFecha);
            }
            placar.push_back(totais.get(j).unwrap());
        }

        if antes.is_none() {
            let ks = Chave::SecoesApuradas(proposta.clone());
            let n: u32 = env.storage().persistent().get(&ks).unwrap_or(0);
            env.storage().persistent().set(&ks, &(n + 1));
            guardar_longo(&env, &ks);
        }
        env.storage()
            .persistent()
            .set(&kr, &(quantas_abertas, placar.clone()));
        guardar_longo(&env, &kr);

        env.events().publish(
            (symbol_short!("apursec"), proposta, secao),
            (quantas_abertas, quantas, placar),
        );
        Ok(quantas_abertas)
    }

    /// **Registra a assinatura da baliza da rodada de abertura.** Qualquer
    /// pessoa, uma vez, depois que a rodada venceu.
    ///
    /// É dela que sai a seção de cada um (DEC-012), e o contrato **confere**
    /// antes de guardar: `e(σ, g₂) == e(H₁(sha256(rodada)), P)`. Sem a
    /// conferência, quem a apresentasse escolheria as seções, que é exatamente o
    /// ataque que a derivação pela baliza fecha.
    ///
    /// A assinatura chega **não comprimida**, 96 bytes, porque é como o host lê
    /// pontos de G1 e descomprimir exigiria raiz quadrada em `Fp`, que ele não
    /// tem. Quem converte é `core::relogio::assinatura_para_host`.
    ///
    /// Custo medido: 24.053.490 instruções, 6% do teto — pago uma vez por
    /// proposta, não por cédula.
    pub fn registrar_abertura(
        env: Env,
        proposta: BytesN<32>,
        assinatura: BytesN<96>,
    ) -> Result<(), Erro> {
        let p = abrir_proposta(&env, &proposta)?;
        if p.rodada_abertura == 0 {
            return Err(Erro::ModoErrado);
        }
        if env.ledger().timestamp() < instante_da_rodada(p.rodada_abertura) {
            return Err(Erro::AberturaAindaNaoVenceu);
        }
        let k = Chave::Abertura(proposta.clone());
        if env.storage().persistent().has(&k) {
            // Já registrada, e ela é única: a assinatura de uma rodada é uma só.
            return Ok(());
        }

        let bls = env.crypto().bls12_381();
        let msg = env
            .crypto()
            .sha256(&Bytes::from_array(&env, &p.rodada_abertura.to_be_bytes()));
        let q = bls.hash_to_g1(
            &Bytes::from_array(&env, &msg.to_array()),
            &Bytes::from_slice(&env, DST_BALIZA),
        );
        let s = Bls12381G1Affine::from_bytes(assinatura.clone());
        let menos_q = bls.g1_mul(&q, &neg_um(&env));

        let mut g1s = Vec::new(&env);
        let mut g2s = Vec::new(&env);
        g1s.push_back(s);
        g1s.push_back(menos_q);
        g2s.push_back(Bls12381G2Affine::from_bytes(BytesN::from_array(
            &env,
            &GERADOR_G2,
        )));
        g2s.push_back(Bls12381G2Affine::from_bytes(BytesN::from_array(
            &env,
            &CHAVE_BALIZA,
        )));
        if !bls.pairing_check(g1s, g2s) {
            return Err(Erro::BalizaNaoConfere);
        }

        env.storage().persistent().set(&k, &assinatura);
        guardar_longo(&env, &k);
        env.events()
            .publish((symbol_short!("abertura"), proposta), p.rodada_abertura);
        Ok(())
    }

    /// A assinatura registrada, para o cliente calcular a própria seção.
    pub fn abertura(env: Env, proposta: BytesN<32>) -> Option<BytesN<96>> {
        env.storage().persistent().get(&Chave::Abertura(proposta))
    }

    /// O que a seção apurou: `(quantas cédulas abriram, totais confidenciais)`.
    pub fn resultado_secao(env: Env, proposta: BytesN<32>, secao: u32) -> Option<(u32, Vec<u32>)> {
        env.storage()
            .persistent()
            .get(&Chave::ResultadoSecao(proposta, secao))
    }

    /// Quantos membros já endossaram exatamente estes números.
    pub fn endossos(
        env: Env,
        proposta: BytesN<32>,
        totais: Vec<u32>,
        aberturas: Vec<Bls12381Fr>,
    ) -> u32 {
        let d = digest_da_apuracao(&env, &totais, &aberturas);
        env.storage()
            .persistent()
            .get(&Chave::Endossos(proposta, d))
            .unwrap_or(0)
    }

    // ===================== leitura =======================================

    pub fn proposta(env: Env, proposta: BytesN<32>) -> Option<Proposta> {
        env.storage().persistent().get(&Chave::Proposta(proposta))
    }

    /// Os acumuladores das perguntas sigilosas, achatados em ordem, para quem
    /// quiser recalcular a apuração por fora.
    pub fn acumulador(env: Env, proposta: BytesN<32>) -> Vec<Bls12381G1Affine> {
        let mut v = Vec::new(&env);
        if let Some(p) = Self::proposta(env.clone(), proposta.clone()) {
            for (q, pg) in p.perguntas.iter().enumerate() {
                if !pg.confidencial {
                    continue;
                }
                for j in 0..pg.opcoes {
                    if let Some(a) = env
                        .storage()
                        .persistent()
                        .get::<_, Bls12381G1Affine>(&Chave::Acum(proposta.clone(), q as u32, j))
                    {
                        v.push_back(a);
                    }
                }
            }
        }
        v
    }

    /// Os totais públicos, achatados sobre **todas** as perguntas em ordem.
    pub fn total_publico(env: Env, proposta: BytesN<32>) -> Vec<u32> {
        let mut v = Vec::new(&env);
        if let Some(p) = Self::proposta(env.clone(), proposta.clone()) {
            for (q, pg) in p.perguntas.iter().enumerate() {
                for j in 0..pg.opcoes {
                    v.push_back(
                        env.storage()
                            .persistent()
                            .get(&Chave::TotalPublico(proposta.clone(), q as u32, j))
                            .unwrap_or(0),
                    );
                }
            }
        }
        v
    }

    pub fn resultado(env: Env, proposta: BytesN<32>) -> Option<Vec<u32>> {
        env.storage().persistent().get(&Chave::Resultado(proposta))
    }

    /// `(confidenciais, públicas)`.
    pub fn comparecimento(env: Env, proposta: BytesN<32>) -> (u32, u32) {
        env.storage()
            .persistent()
            .get(&Chave::Comparecimento(proposta))
            .unwrap_or((0, 0))
    }

    pub fn ja_votou(env: Env, proposta: BytesN<32>, votante: Address) -> bool {
        env.storage()
            .persistent()
            .has(&Chave::Votou(proposta, votante))
    }

    /// **O caderno, lido.** Quem compareceu — e, por subtração da lista de
    /// aptos, quem faltou. É a leitura que o voto obrigatório precisa, e a
    /// única sobre uma pessoa que esta proposta responde.
    pub fn compareceu(env: Env, proposta: BytesN<32>, votante: Address) -> bool {
        env.storage()
            .persistent()
            .has(&Chave::Compareceu(proposta, votante))
    }

    /// Quantas seções existem agora. Na aberta cresce com o comparecimento.
    pub fn secoes(env: Env, proposta: BytesN<32>) -> u32 {
        match env
            .storage()
            .persistent()
            .get::<Chave, Proposta>(&Chave::Proposta(proposta.clone()))
        {
            Some(p) => quantas_secoes(&env, &proposta, &p),
            None => 0,
        }
    }

    /// Em que seção a pessoa caiu. Na votação aberta é o contrato que decide,
    /// então quem vota precisa perguntar — e perguntar à cadeia, não ao
    /// navegador, que pode ter sido trocado.
    pub fn secao_de(env: Env, proposta: BytesN<32>, votante: Address) -> Option<u32> {
        env.storage()
            .persistent()
            .get(&Chave::Compareceu(proposta, votante))
    }

    /// **O anel**, na ordem em que as pessoas compareceram.
    ///
    /// Quem vota precisa dele inteiro para assinar: anonimato de anel é
    /// anonimato dentro de um conjunto conhecido, e o conjunto tem de ser
    /// conhecido. Devolver isto não entrega nada — a ordem é a de chegada ao
    /// caderno, que já é pública.
    pub fn anel(env: Env, proposta: BytesN<32>, secao: u32) -> Vec<Bls12381G1Affine> {
        env.storage()
            .persistent()
            .get(&Chave::Anel(proposta, secao))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// `Hp` desta proposta, para o cliente assinar o anel.
    ///
    /// **Sem isto o dapp não existe.** `Hp` vem de `hash_to_g1`, que é função do
    /// host; reimplementá-la no cliente byte a byte seria pedir para divergir, e
    /// uma divergência aqui faz toda assinatura falhar sem dizer por quê. O
    /// mesmo motivo de `gerador_h` existir.
    pub fn hp(env: Env, proposta: BytesN<32>) -> Bls12381G1Affine {
        calcular_hp(&env, &proposta)
    }

    /// `H`, para o cliente montar compromissos sem recalcular hash-to-curve.
    pub fn gerador_h(env: Env) -> Bls12381G1Affine {
        env.storage()
            .instance()
            .get(&Instancia::GeradorH)
            .unwrap_or_else(|| calcular_h(&env))
    }
}

/// Uma `raiz_aptos` de 32 zeros quer dizer **votação aberta**: não há lista, e
/// qualquer carteira comparece.
///
/// Zeros são seguros como sentinela porque a raiz real é um SHA-256 de folhas
/// com separação de domínio — acertar 32 zeros exigiria inverter o hash.
///
/// O que a votação aberta perde está declarado e não é pouco: sem lista não há
/// `aptos − compareceram`, logo não há voto obrigatório, e nada impede a mesma
/// pessoa de comparecer com cinquenta carteiras. O que ela mantém é o que o
/// projeto existe para provar: ninguém descobre a escolha de ninguém, e nada
/// liga pessoa a cédula.
/// Quantas seções existem **agora**.
///
/// Na fechada é o que foi fixado na abertura. Na aberta cresce com o caderno:
/// `teto(compareceram / limite)`, mínimo 1. `p.secoes` sozinho mentiria, porque
/// ele fica parado em 1 enquanto as seções abrem.
fn quantas_secoes(env: &Env, proposta: &BytesN<32>, p: &Proposta) -> u32 {
    if p.limite_secao == 0 {
        return p.secoes;
    }
    let n: u32 = env
        .storage()
        .persistent()
        .get(&Chave::Caderno(proposta.clone()))
        .unwrap_or(0);
    if n == 0 {
        1
    } else {
        n.div_ceil(p.limite_secao)
    }
}

fn e_aberta(raiz: &BytesN<32>) -> bool {
    raiz == &BytesN::from_array(raiz.env(), &[0u8; 32])
}

// ===================== auxiliares ========================================

/// O miolo de uma cédula: conferir tudo, e só depois somar.
///
/// `identidade` é o que prende as provas a quem vota — o XDR do endereço no
/// voto identificado, a imagem de chave no voto em anel. É o único ponto em que
/// os dois modos divergem, e por isso o resto é compartilhado: duas cópias
/// desta função divergiriam, e a divergência seria silenciosa.
#[allow(clippy::too_many_arguments)]
fn conferir_e_somar(
    env: &Env,
    p: &Proposta,
    proposta: &BytesN<32>,
    identidade: &Bytes,
    compromissos: &Vec<Bls12381G1Affine>,
    provas: &Vec<ProvaCds>,
    provas_soma: &Vec<ProvaSoma>,
    escolhas: &Vec<u32>,
    peso: u32,
) -> Result<(), Erro> {
    let g = gerador_g(env);
    let h: Bls12381G1Affine = env
        .storage()
        .instance()
        .get(&Instancia::GeradorH)
        .ok_or(Erro::PropostaNaoExiste)?;
    let bls = env.crypto().bls12_381();

    let mut off_conf = 0u32; // posição em `compromissos`/`provas`
    let mut off_publ = 0u32; // posição em `escolhas`
    let mut i_soma = 0u32; // qual prova de soma
    for (q, pg) in p.perguntas.iter().enumerate() {
        let q = q as u32;
        if !pg.confidencial {
            conferir_bloco(escolhas, off_publ, pg.opcoes, peso)?;
            off_publ += pg.opcoes;
            continue;
        }

        // Todo ponto que chega é validado: o host não valida sozinho, e
        // um ponto de ordem pequena vazaria informação sobre o escalar
        // (sonda 10).
        let mut soma = infinito(env);
        for j in 0..pg.opcoes {
            let c = compromissos.get(off_conf + j).unwrap();
            let pr = provas.get(off_conf + j).unwrap();
            validar(env, &c)?;
            validar(env, &pr.a0)?;
            validar(env, &pr.a1)?;

            let ctx = contexto_de(env, proposta, identidade, q, j);
            if !verificar_cds(env, &ctx, &g, &h, &c, &pr) {
                return Err(Erro::ProvaBinariaInvalida);
            }
            soma = bls.g1_add(&soma, &c);
        }

        // D = (Σ C_j da pergunta) − w·G tem de ser múltiplo conhecido de
        // H. Com cada v_j ∈ {0,1} pelas disjuntivas, isso fecha a boa
        // formação **desta** pergunta.
        let ps = provas_soma.get(i_soma).unwrap();
        validar(env, &ps.a)?;
        let d = bls.g1_add(&soma, &(-bls.g1_mul(&g, &fr(env, peso))));
        let ctx = contexto_de(env, proposta, identidade, q, u32::MAX);
        if !verificar_soma(env, &ctx, &h, &d, &ps) {
            return Err(Erro::ProvaDeSomaInvalida);
        }
        i_soma += 1;
        off_conf += pg.opcoes;
    }

    // Daqui para baixo a cédula inteira já passou. Só agora se escreve —
    // uma pergunta mal formada não deixa rastro parcial no acumulador.
    let mut off_conf = 0u32;
    let mut off_publ = 0u32;
    for (q, pg) in p.perguntas.iter().enumerate() {
        let q = q as u32;
        for j in 0..pg.opcoes {
            if pg.confidencial {
                // Agregação homomórfica: g1_add é ~30× mais barato que
                // g1_mul, então somar é praticamente de graça.
                let k = Chave::Acum(proposta.clone(), q, j);
                let a: Bls12381G1Affine = env.storage().persistent().get(&k).unwrap();
                let c = compromissos.get(off_conf + j).unwrap();
                env.storage().persistent().set(&k, &bls.g1_add(&a, &c));
                guardar_longo(env, &k);
            } else {
                let k = Chave::TotalPublico(proposta.clone(), q, j);
                let t: u32 = env.storage().persistent().get(&k).unwrap();
                env.storage()
                    .persistent()
                    .set(&k, &(t + escolhas.get(off_publ + j).unwrap()));
                guardar_longo(env, &k);
            }
        }
        if pg.confidencial {
            off_conf += pg.opcoes;
        } else {
            off_publ += pg.opcoes;
        }
    }

    let kc = Chave::Comparecimento(proposta.clone());
    let (conf, publ): (u32, u32) = env.storage().persistent().get(&kc).unwrap();
    env.storage().persistent().set(&kc, &(conf + 1, publ));
    guardar_longo(env, &kc);

    Ok(())
}

/// Estende o TTL ao teto da rede. Aplicado a tudo que o verificador precisará
/// depois do sétimo dia, e **não** às entradas `Votou`: estender 10.000 delas
/// custaria ~2.070 XLM, e elas só impedem voto duplo *durante* a votação.
/// Gênese e período da cadeia `quicknet` da baliza, e o tamanho do criptograma.
/// Formato congelado (SPEC §5), medidos em 2026-10-07.
///
/// Repetidos aqui em vez de importados do `core` porque o `core` é
/// dev-dependency: trazê-lo para o runtime arrastaria arkworks para dentro do
/// Wasm por causa de três linhas de aritmética. O risco de repetir é divergir,
/// e é por isso que `a_rodada_do_contrato_bate_com_a_do_core` cruza as duas a
/// cada `cargo test` — o mesmo padrão que faz o contrato verificar as provas que
/// o `core` gera, em vez de confiar num vetor congelado.
const GENESE_BALIZA: u64 = 1_692_803_367;
const PERIODO_BALIZA: u64 = 3;
pub const TAMANHO_CRIPTOGRAMA: u32 = 160;

/// O DST do esquema `bls-unchained-g1-rfc9380`. É da drand, não nosso.
const DST_BALIZA: &[u8] = b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_";

/// `-1` em `Fr`, para negar um ponto: `e(σ, g₂) · e(−H₁, P) == 1`.
fn neg_um(env: &Env) -> Bls12381Fr {
    let bls = env.crypto().bls12_381();
    bls.fr_sub(&fr(env, 0), &fr(env, 1))
}

/// O gerador de G2 e a chave pública da baliza, nos 192 bytes que o host lê:
/// `be(x.c1) ‖ be(x.c0) ‖ be(y.c1) ‖ be(y.c0)`, não comprimido.
///
/// A ordem de Fp2 não foi deduzida — está medida em
/// `o_host_confere_a_baliza_e_a_ordem_de_g2_e_medida`, que com a ordem trocada
/// vê o host recusar o ponto. Vêm de `core::relogio`, e o mesmo teste as cruza.
const GERADOR_G2: [u8; 192] = [
    0x13, 0xe0, 0x2b, 0x60, 0x52, 0x71, 0x9f, 0x60, 0x7d, 0xac, 0xd3, 0xa0, 0x88, 0x27, 0x4f, 0x65,
    0x59, 0x6b, 0xd0, 0xd0, 0x99, 0x20, 0xb6, 0x1a, 0xb5, 0xda, 0x61, 0xbb, 0xdc, 0x7f, 0x50, 0x49,
    0x33, 0x4c, 0xf1, 0x12, 0x13, 0x94, 0x5d, 0x57, 0xe5, 0xac, 0x7d, 0x05, 0x5d, 0x04, 0x2b, 0x7e,
    0x02, 0x4a, 0xa2, 0xb2, 0xf0, 0x8f, 0x0a, 0x91, 0x26, 0x08, 0x05, 0x27, 0x2d, 0xc5, 0x10, 0x51,
    0xc6, 0xe4, 0x7a, 0xd4, 0xfa, 0x40, 0x3b, 0x02, 0xb4, 0x51, 0x0b, 0x64, 0x7a, 0xe3, 0xd1, 0x77,
    0x0b, 0xac, 0x03, 0x26, 0xa8, 0x05, 0xbb, 0xef, 0xd4, 0x80, 0x56, 0xc8, 0xc1, 0x21, 0xbd, 0xb8,
    0x06, 0x06, 0xc4, 0xa0, 0x2e, 0xa7, 0x34, 0xcc, 0x32, 0xac, 0xd2, 0xb0, 0x2b, 0xc2, 0x8b, 0x99,
    0xcb, 0x3e, 0x28, 0x7e, 0x85, 0xa7, 0x63, 0xaf, 0x26, 0x74, 0x92, 0xab, 0x57, 0x2e, 0x99, 0xab,
    0x3f, 0x37, 0x0d, 0x27, 0x5c, 0xec, 0x1d, 0xa1, 0xaa, 0xa9, 0x07, 0x5f, 0xf0, 0x5f, 0x79, 0xbe,
    0x0c, 0xe5, 0xd5, 0x27, 0x72, 0x7d, 0x6e, 0x11, 0x8c, 0xc9, 0xcd, 0xc6, 0xda, 0x2e, 0x35, 0x1a,
    0xad, 0xfd, 0x9b, 0xaa, 0x8c, 0xbd, 0xd3, 0xa7, 0x6d, 0x42, 0x9a, 0x69, 0x51, 0x60, 0xd1, 0x2c,
    0x92, 0x3a, 0xc9, 0xcc, 0x3b, 0xac, 0xa2, 0x89, 0xe1, 0x93, 0x54, 0x86, 0x08, 0xb8, 0x28, 0x01,
];

/// A chave pública da cadeia `quicknet`, idem.
const CHAVE_BALIZA: [u8; 192] = [
    0x03, 0xcf, 0x0f, 0x28, 0x96, 0xad, 0xee, 0x7e, 0xb8, 0xb5, 0xf0, 0x1f, 0xca, 0xd3, 0x91, 0x22,
    0x12, 0xc4, 0x37, 0xe0, 0x07, 0x3e, 0x91, 0x1f, 0xb9, 0x00, 0x22, 0xd3, 0xe7, 0x60, 0x18, 0x3c,
    0x8c, 0x4b, 0x45, 0x0b, 0x6a, 0x0a, 0x6c, 0x3a, 0xc6, 0xa5, 0x77, 0x6a, 0x2d, 0x10, 0x64, 0x51,
    0x0d, 0x1f, 0xec, 0x75, 0x8c, 0x92, 0x1c, 0xc2, 0x2b, 0x0e, 0x17, 0xe6, 0x3a, 0xaf, 0x4b, 0xcb,
    0x5e, 0xd6, 0x63, 0x04, 0xde, 0x9c, 0xf8, 0x09, 0xbd, 0x27, 0x4c, 0xa7, 0x3b, 0xab, 0x4a, 0xf5,
    0xa6, 0xe9, 0xc7, 0x6a, 0x4b, 0xc0, 0x9e, 0x76, 0xea, 0xe8, 0x99, 0x1e, 0xf5, 0xec, 0xe4, 0x5a,
    0x01, 0xa7, 0x14, 0xf2, 0xed, 0xb7, 0x41, 0x19, 0xa2, 0xf2, 0xb0, 0xd5, 0xa7, 0xc7, 0x5b, 0xa9,
    0x02, 0xd1, 0x63, 0x70, 0x0a, 0x61, 0xbc, 0x22, 0x4e, 0xde, 0xdd, 0x8e, 0x63, 0xae, 0xf7, 0xbe,
    0x1a, 0xaf, 0x8e, 0x93, 0xd7, 0xa9, 0x71, 0x8b, 0x04, 0x7c, 0xcd, 0xdb, 0x3e, 0xb5, 0xd6, 0x8b,
    0x0e, 0x5d, 0xb2, 0xb6, 0xbf, 0xbb, 0x01, 0xc8, 0x67, 0x74, 0x9c, 0xad, 0xff, 0xca, 0x88, 0xb3,
    0x6c, 0x24, 0xf3, 0x01, 0x2b, 0xa0, 0x9f, 0xc4, 0xd3, 0x02, 0x2c, 0x5c, 0x37, 0xdc, 0xe0, 0xf9,
    0x77, 0xd3, 0xad, 0xb5, 0xd1, 0x83, 0xc7, 0x47, 0x7c, 0x44, 0x2b, 0x1f, 0x04, 0x51, 0x52, 0x73,
];

/// O instante em que a rodada vence. É o prazo, não um campo separado.
fn instante_da_rodada(rodada: u64) -> u64 {
    GENESE_BALIZA + (rodada - 1) * PERIODO_BALIZA
}

/// A cadeia daquela seção, ou o começo: 32 zeros e nenhuma cédula.
fn cadeia_de(env: &Env, proposta: &BytesN<32>, secao: u32) -> (BytesN<32>, u32) {
    env.storage()
        .persistent()
        .get(&Chave::Cadeia(proposta.clone(), secao))
        .unwrap_or((BytesN::from_array(env, &[0u8; 32]), 0))
}

fn guardar_longo(env: &Env, k: &Chave) {
    let m = env.storage().max_ttl();
    env.storage().persistent().extend_ttl(k, m - 1, m);
}

/// **O ponto no infinito, como o host o codifica.** Não é 96 bytes de zero —
/// isso o host recusa com "point not on curve". É o bit de flag do formato
/// zcash (`0x40` no byte alto), achado empiricamente e travado em teste. O
/// acumulador de uma proposta sem votos é o infinito, então errar aqui
/// quebraria a primeira cédula de toda votação.
fn infinito(env: &Env) -> Bls12381G1Affine {
    let mut b = [0u8; 96];
    b[0] = 0x40;
    Bls12381G1Affine::from_bytes(BytesN::from_array(env, &b))
}

/// `sha256(totais ‖ aberturas)` — o que um membro da mesa endossa.
///
/// Prender o endosso ao digest é o que impede um membro de autorizar "a
/// apuração" e a mesa publicar outros números. Discordar é simplesmente não
/// endossar aquele digest.
fn digest_da_apuracao(env: &Env, totais: &Vec<u32>, aberturas: &Vec<Bls12381Fr>) -> BytesN<32> {
    let mut b = Bytes::from_slice(env, b"TESSERA-V1-APURACAO");
    for t in totais.iter() {
        b.extend_from_array(&t.to_be_bytes());
    }
    for a in aberturas.iter() {
        b.extend_from_array(&a.to_bytes().to_array());
    }
    env.crypto().sha256(&b).to_bytes()
}

/// O formato da cédula: `(opções confidenciais, opções públicas, perguntas
/// confidenciais)`. É o que diz quantos argumentos cada chamada tem de trazer.
fn formato(p: &Proposta) -> (u32, u32, u32) {
    let (mut conf, mut publ, mut perg) = (0u32, 0u32, 0u32);
    for q in p.perguntas.iter() {
        if q.confidencial {
            conf += q.opcoes;
            perg += 1;
        } else {
            publ += q.opcoes;
        }
    }
    (conf, publ, perg)
}

/// As duas regras de uma pergunta respondida em claro: cada opção é binária e
/// a pergunta soma exatamente o peso.
///
/// É o espelho visível do que a disjuntiva e a prova de soma fazem na pergunta
/// sigilosa — e é **por pergunta**, nunca sobre a cédula inteira, pelo mesmo
/// motivo que a prova de soma é por pergunta.
fn conferir_bloco(escolhas: &Vec<u32>, desde: u32, opcoes: u32, peso: u32) -> Result<(), Erro> {
    let mut total = 0u32;
    for j in 0..opcoes {
        let v = escolhas.get(desde + j).unwrap();
        if v > 1 {
            return Err(Erro::EscolhaForaDoBinario);
        }
        total += v;
    }
    if total != peso {
        return Err(Erro::SomaDiferenteDoPeso);
    }
    Ok(())
}

fn abrir_proposta(env: &Env, proposta: &BytesN<32>) -> Result<Proposta, Erro> {
    env.storage()
        .persistent()
        .get(&Chave::Proposta(proposta.clone()))
        .ok_or(Erro::PropostaNaoExiste)
}

/// Aptidão e peso, numa checagem só.
///
/// A v1 exige peso 1. Pesos públicos distintos são quebrados por
/// subconjunto-soma (PROTOCOLO §6.3) e o contrato **recusa** a configuração em vez
/// de documentá-la como cuidado. O campo existe na folha para a v1.1, onde ele
/// vira identificador de faixa com ocupação mínima τ.
fn conferir_aptidao(
    env: &Env,
    p: &Proposta,
    votante: &Address,
    peso: u32,
    indice: u32,
    caminho: &Vec<BytesN<32>>,
) -> Result<(), Erro> {
    if peso != 1 {
        return Err(Erro::PesoNaoUnitario);
    }
    if !verificar_aptidao(env, votante, peso, indice, caminho, &p.raiz_aptos) {
        return Err(Erro::NaoEstaNaListaDeAptos);
    }
    Ok(())
}

fn marcar_votou(env: &Env, proposta: &BytesN<32>, votante: &Address) -> Result<(), Erro> {
    let k = Chave::Votou(proposta.clone(), votante.clone());
    if env.storage().persistent().has(&k) {
        return Err(Erro::JaVotou);
    }
    // TTL padrão de propósito: ver `guardar_longo`.
    env.storage().persistent().set(&k, &true);
    Ok(())
}

#[cfg(test)]
mod test;
