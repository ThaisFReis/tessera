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

mod cripto;
mod tipos;

pub use tipos::{Erro, Pergunta, Proposta, ProvaCds, ProvaSoma};

use cripto::*;
use soroban_sdk::{
    contract, contractimpl, symbol_short,
    crypto::bls12_381::{Bls12381Fr, Bls12381G1Affine},
    Address, Bytes, BytesN, Env, Vec,
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
    ) -> Result<(), Erro> {
        governanca.require_auth();

        if env.storage().persistent().has(&Chave::Proposta(proposta.clone())) {
            return Err(Erro::PropostaJaExiste);
        }
        if perguntas.len() == 0 || perguntas.len() > MAX_PERGUNTAS {
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
        if limiar == 0 || limiar > mesa.len() {
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

        let n_perguntas = perguntas.len();
        let p = Proposta { perguntas, raiz_aptos, mesa, limiar, abre_em, fecha_em };
        env.storage().persistent().set(&Chave::Proposta(proposta.clone()), &p);
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
            (n_perguntas, conf_opcoes, abre_em, fecha_em, limiar),
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
        conferir_aptidao(&env, &p, &votante, peso, indice, &caminho)?;
        marcar_votou(&env, &proposta, &votante)?;

        let g = gerador_g(&env);
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
                conferir_bloco(&escolhas, off_publ, pg.opcoes, peso)?;
                off_publ += pg.opcoes;
                continue;
            }

            // Todo ponto que chega é validado: o host não valida sozinho, e
            // um ponto de ordem pequena vazaria informação sobre o escalar
            // (sonda 10).
            let mut soma = infinito(&env);
            for j in 0..pg.opcoes {
                let c = compromissos.get(off_conf + j).unwrap();
                let pr = provas.get(off_conf + j).unwrap();
                validar(&env, &c)?;
                validar(&env, &pr.a0)?;
                validar(&env, &pr.a1)?;

                let ctx = contexto(&env, &proposta, &votante, q, j);
                if !verificar_cds(&env, &ctx, &g, &h, &c, &pr) {
                    return Err(Erro::ProvaBinariaInvalida);
                }
                soma = bls.g1_add(&soma, &c);
            }

            // D = (Σ C_j da pergunta) − w·G tem de ser múltiplo conhecido de
            // H. Com cada v_j ∈ {0,1} pelas disjuntivas, isso fecha a boa
            // formação **desta** pergunta.
            let ps = provas_soma.get(i_soma).unwrap();
            validar(&env, &ps.a)?;
            let d = bls.g1_add(&soma, &(-bls.g1_mul(&g, &fr(&env, peso))));
            let ctx = contexto(&env, &proposta, &votante, q, u32::MAX);
            if !verificar_soma(&env, &ctx, &h, &d, &ps) {
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
                    guardar_longo(&env, &k);
                } else {
                    let k = Chave::TotalPublico(proposta.clone(), q, j);
                    let t: u32 = env.storage().persistent().get(&k).unwrap();
                    env.storage()
                        .persistent()
                        .set(&k, &(t + escolhas.get(off_publ + j).unwrap()));
                    guardar_longo(&env, &k);
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
        guardar_longo(&env, &kc);

        // O evento é o que o nível 2 do verificador lê dos arquivos de
        // histórico para recalcular o acumulador sem depender do RPC (§8.3).
        env.events().publish(
            (symbol_short!("votar"), proposta, votante),
            (compromissos, escolhas),
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
    // Nenhuma eleição séria aceita isso. O Brasil protege a célula pequena
    // **antes**, agregando seções com menos de 50 eleitores, e nunca recusando
    // a contagem depois (TSE, Res. 23.669/2021). Sem revelação individual não
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
    /// `r` individual se junta em lugar algum (SPEC §4.3).
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
        if env.storage().persistent().has(&Chave::Resultado(proposta.clone())) {
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

        // **A regra de τ.** Ver SPEC §6.6. Ou ninguém votou em sigilo — e não
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
                        .get::<_, Bls12381G1Affine>(&Chave::Acum(
                            proposta.clone(),
                            q as u32,
                            j,
                        ))
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
        env.storage().persistent().has(&Chave::Votou(proposta, votante))
    }

    /// `H`, para o cliente montar compromissos sem recalcular hash-to-curve.
    pub fn gerador_h(env: Env) -> Bls12381G1Affine {
        env.storage()
            .instance()
            .get(&Instancia::GeradorH)
            .unwrap_or_else(|| calcular_h(&env))
    }
}

// ===================== auxiliares ========================================

/// Estende o TTL ao teto da rede. Aplicado a tudo que o verificador precisará
/// depois do sétimo dia, e **não** às entradas `Votou`: estender 10.000 delas
/// custaria ~2.070 XLM, e elas só impedem voto duplo *durante* a votação.
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
/// subconjunto-soma (SPEC §6.3) e o contrato **recusa** a configuração em vez
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
