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

pub use tipos::{Erro, Proposta, ProvaCds, ProvaSoma};

use cripto::*;
use soroban_sdk::{
    contract, contractimpl, symbol_short,
    crypto::bls12_381::{Bls12381Fr, Bls12381G1Affine},
    Address, BytesN, Env, Vec,
};
use tipos::{Chave, Instancia, MAX_OPCOES, TAU};

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
    pub fn abrir(
        env: Env,
        governanca: Address,
        proposta: BytesN<32>,
        opcoes: u32,
        raiz_aptos: BytesN<32>,
        mesa: Vec<Address>,
        limiar: u32,
        fecha_em: u32,
    ) -> Result<(), Erro> {
        governanca.require_auth();

        if env.storage().persistent().has(&Chave::Proposta(proposta.clone())) {
            return Err(Erro::PropostaJaExiste);
        }
        if opcoes < 2 || opcoes > MAX_OPCOES {
            return Err(Erro::OpcoesForaDaFaixa);
        }
        if limiar == 0 || limiar > mesa.len() {
            return Err(Erro::LimiarInvalido);
        }
        if fecha_em <= env.ledger().sequence() {
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

        let p = Proposta { opcoes, raiz_aptos, mesa, limiar, fecha_em };
        env.storage().persistent().set(&Chave::Proposta(proposta.clone()), &p);
        guardar_longo(&env, &Chave::Proposta(proposta.clone()));

        // Acumuladores começam no infinito: o compromisso de ninguém.
        let zero = infinito(&env);
        for j in 0..opcoes {
            let k = Chave::Acum(proposta.clone(), j);
            env.storage().persistent().set(&k, &zero);
            guardar_longo(&env, &k);
            let kp = Chave::TotalPublico(proposta.clone(), j);
            env.storage().persistent().set(&kp, &0u32);
            guardar_longo(&env, &kp);
        }
        let kc = Chave::Comparecimento(proposta.clone());
        env.storage().persistent().set(&kc, &(0u32, 0u32));
        guardar_longo(&env, &kc);

        env.events().publish(
            (symbol_short!("abrir"), proposta),
            (opcoes, fecha_em, limiar),
        );
        Ok(())
    }

    // ===================== votar =========================================

    /// Registra uma cédula confidencial. Quem vota paga a taxa.
    ///
    /// O que entra no ledger é `C_j = v_j·G + r_j·H` por opção, e provas de que
    /// a cédula é bem formada. O `v_j` não entra, não transita, e é destruído
    /// no cliente.
    ///
    /// Custo medido: **33.480.865 instruções para `m = 2`, 8,4% do teto.**
    pub fn votar(
        env: Env,
        proposta: BytesN<32>,
        votante: Address,
        compromissos: Vec<Bls12381G1Affine>,
        provas: Vec<ProvaCds>,
        prova_soma: ProvaSoma,
        caminho: Vec<BytesN<32>>,
        indice: u32,
        peso: u32,
    ) -> Result<(), Erro> {
        votante.require_auth();
        let p = abrir_proposta(&env, &proposta)?;

        if env.ledger().sequence() >= p.fecha_em {
            return Err(Erro::VotacaoEncerrada);
        }
        if compromissos.len() != p.opcoes || provas.len() != p.opcoes {
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

        // Todo ponto que chega é validado: o host não valida sozinho, e um
        // ponto de ordem pequena vazaria informação sobre o escalar (sonda 10).
        let mut soma = infinito(&env);
        for j in 0..p.opcoes {
            let c = compromissos.get(j).unwrap();
            let pr = provas.get(j).unwrap();
            validar(&env, &c)?;
            validar(&env, &pr.a0)?;
            validar(&env, &pr.a1)?;

            let ctx = contexto(&env, &proposta, &votante, j);
            if !verificar_cds(&env, &ctx, &g, &h, &c, &pr) {
                return Err(Erro::ProvaBinariaInvalida);
            }
            soma = bls.g1_add(&soma, &c);
        }

        // D = (Σ C_j) − w·G tem de ser um múltiplo conhecido de H. Como cada
        // v_j ∈ {0,1} pelas disjuntivas, isso fecha a boa formação da cédula.
        validar(&env, &prova_soma.a)?;
        let d = bls.g1_add(&soma, &(-bls.g1_mul(&g, &fr(&env, peso))));
        let ctx = contexto(&env, &proposta, &votante, u32::MAX);
        if !verificar_soma(&env, &ctx, &h, &d, &prova_soma) {
            return Err(Erro::ProvaDeSomaInvalida);
        }

        // Agregação homomórfica: g1_add é ~30× mais barato que g1_mul, então
        // somar é praticamente de graça. O custo está todo na verificação.
        for j in 0..p.opcoes {
            let k = Chave::Acum(proposta.clone(), j);
            let a: Bls12381G1Affine = env.storage().persistent().get(&k).unwrap();
            env.storage()
                .persistent()
                .set(&k, &bls.g1_add(&a, &compromissos.get(j).unwrap()));
            guardar_longo(&env, &k);
        }

        let kc = Chave::Comparecimento(proposta.clone());
        let (conf, publ): (u32, u32) = env.storage().persistent().get(&kc).unwrap();
        env.storage().persistent().set(&kc, &(conf + 1, publ));
        guardar_longo(&env, &kc);

        // O evento é o que o nível 2 do verificador lê dos arquivos de
        // histórico para recalcular o acumulador sem depender do RPC (§8.3).
        env.events().publish(
            (symbol_short!("votar"), proposta, votante),
            compromissos,
        );
        Ok(())
    }

    /// Registra uma cédula **pública**: a escolha vai em claro para o ledger.
    ///
    /// Sugestão de um membro da SDF, e encaixa sem primitiva nova — um campo
    /// público é apenas um compromisso cuja abertura é revelada, e revelar a
    /// abertura torna o compromisso redundante.
    ///
    /// **Não é gratuito.** Toda informação pública particiona o conjunto de
    /// anonimato (SPEC §6.6): 50 votantes, 48 públicos e o total conhecido
    /// determinam os 2 confidenciais por subtração. Por isso `apurar()` recusa
    /// publicar com menos de `TAU` cédulas confidenciais.
    pub fn votar_publico(
        env: Env,
        proposta: BytesN<32>,
        votante: Address,
        escolhas: Vec<u32>,
        caminho: Vec<BytesN<32>>,
        indice: u32,
        peso: u32,
    ) -> Result<(), Erro> {
        votante.require_auth();
        let p = abrir_proposta(&env, &proposta)?;

        if env.ledger().sequence() >= p.fecha_em {
            return Err(Erro::VotacaoEncerrada);
        }
        if escolhas.len() != p.opcoes {
            return Err(Erro::ArgumentoMalFormado);
        }
        conferir_aptidao(&env, &p, &votante, peso, indice, &caminho)?;
        marcar_votou(&env, &proposta, &votante)?;

        // As mesmas regras da cédula confidencial, só que conferíveis a olho:
        // cada escolha é binária e a soma é o peso.
        let mut total = 0u32;
        for v in escolhas.iter() {
            if v > 1 {
                return Err(Erro::EscolhaForaDoBinario);
            }
            total += v;
        }
        if total != peso {
            return Err(Erro::SomaDiferenteDoPeso);
        }

        for j in 0..p.opcoes {
            let k = Chave::TotalPublico(proposta.clone(), j);
            let t: u32 = env.storage().persistent().get(&k).unwrap();
            env.storage().persistent().set(&k, &(t + escolhas.get(j).unwrap()));
            guardar_longo(&env, &k);
        }

        let kc = Chave::Comparecimento(proposta.clone());
        let (conf, publ): (u32, u32) = env.storage().persistent().get(&kc).unwrap();
        env.storage().persistent().set(&kc, &(conf, publ + 1));
        guardar_longo(&env, &kc);

        env.events()
            .publish((symbol_short!("voto_pub"), proposta, votante), escolhas);
        Ok(())
    }

    // ===================== apurar ========================================

    /// Apura. Chamada por `k` membros da mesa, que assinam juntos.
    ///
    /// A mesa publica `(T_j, R_j)` e o contrato **confere** `A_j == T_j·G +
    /// R_j·H`. Não procura o total: a sonda 5 mostrou que procurar por força
    /// bruta custa 124.277 por unidade de peso e tem teto de ~3.218, enquanto
    /// conferir custa 5.408.931 **constante no comparecimento** — medido
    /// idêntico para 250 e para 1.000.000.
    ///
    /// Revelar `R_j = Σ r_{i,j}` não revela nenhum `r_{i,j}`: é a soma de `n`
    /// valores uniformes, e conhecer a soma de `n` incógnitas não determina
    /// nenhuma delas. E `R_j` chega à mesa por shares de Shamir, então nenhum
    /// `r` individual se junta em lugar algum (SPEC §4.3).
    pub fn apurar(
        env: Env,
        proposta: BytesN<32>,
        assinantes: Vec<Address>,
        totais: Vec<u32>,
        aberturas: Vec<Bls12381Fr>,
    ) -> Result<Vec<u32>, Erro> {
        let p = abrir_proposta(&env, &proposta)?;

        if env.ledger().sequence() < p.fecha_em {
            return Err(Erro::VotacaoAindaAberta);
        }
        if env.storage().persistent().has(&Chave::Resultado(proposta.clone())) {
            return Err(Erro::JaApurada);
        }
        if totais.len() != p.opcoes || aberturas.len() != p.opcoes {
            return Err(Erro::ArgumentoMalFormado);
        }

        // k-de-N: todo assinante tem de ser membro, distinto, e autorizar.
        if assinantes.len() < p.limiar {
            return Err(Erro::MesaAbaixoDoLimiar);
        }
        for (i, a) in assinantes.iter().enumerate() {
            if !p.mesa.iter().any(|m| m == a) {
                return Err(Erro::NaoEMembroDaMesa);
            }
            if assinantes.iter().take(i).any(|o| o == a) {
                return Err(Erro::MembroRepetido);
            }
            a.require_auth();
        }

        // **A regra de τ.** Ver SPEC §6.6. Ou ninguém votou em sigilo — e não
        // há sigilo a proteger — ou pelo menos τ votaram. Uma coligação que
        // publique os próprios votos de propósito encolheria o conjunto secreto
        // até determiná-lo; aqui ela trava a apuração em vez de ler os votos.
        // A troca é deliberada: uma votação travada é contestável e repetível,
        // um voto vazado não volta atrás.
        let (conf, _publ): (u32, u32) = env
            .storage()
            .persistent()
            .get(&Chave::Comparecimento(proposta.clone()))
            .unwrap();
        if conf > 0 && conf < TAU {
            return Err(Erro::AnonimatoInsuficiente);
        }

        // Como a v1 é um-voto-por-pessoa, a soma dos totais confidenciais tem
        // de ser exatamente o número de cédulas confidenciais. Pega uma mesa
        // que invente votos sem nem precisar abrir o acumulador.
        let mut soma_t = 0u32;
        for t in totais.iter() {
            soma_t += t;
        }
        if soma_t != conf {
            return Err(Erro::TotalDiferenteDoComparecimento);
        }

        let g = gerador_g(&env);
        let h: Bls12381G1Affine = env
            .storage()
            .instance()
            .get(&Instancia::GeradorH)
            .ok_or(Erro::PropostaNaoExiste)?;
        let bls = env.crypto().bls12_381();

        let mut resultado = Vec::new(&env);
        for j in 0..p.opcoes {
            let a: Bls12381G1Affine = env
                .storage()
                .persistent()
                .get(&Chave::Acum(proposta.clone(), j))
                .unwrap();

            // A_j == T_j·G + R_j·H, um MSM de 2 termos. 5.408.931 medidos.
            let mut ps = Vec::new(&env);
            let mut ss = Vec::new(&env);
            ps.push_back(g.clone());
            ps.push_back(h.clone());
            ss.push_back(fr(&env, totais.get(j).unwrap()));
            ss.push_back(aberturas.get(j).unwrap());
            if bls.g1_msm(ps, ss) != a {
                return Err(Erro::AberturaNaoFecha);
            }

            let publico: u32 = env
                .storage()
                .persistent()
                .get(&Chave::TotalPublico(proposta.clone(), j))
                .unwrap();
            resultado.push_back(totais.get(j).unwrap() + publico);
        }

        let k = Chave::Resultado(proposta.clone());
        env.storage().persistent().set(&k, &resultado);
        guardar_longo(&env, &k);

        env.events().publish(
            (symbol_short!("apurar"), proposta),
            (totais, aberturas, resultado.clone()),
        );
        Ok(resultado)
    }

    /// Paga o aluguel do próprio contrato: estende instância e código ao teto
    /// da rede. **Qualquer pessoa chama**, e é um bem público — enquanto
    /// alguém pagar, todas as propostas continuam utilizáveis.
    ///
    /// Custa ~181 XLM por 180 dias, porque o aluguel é proporcional ao tamanho
    /// e o Wasm tem 21 KB. É um custo de operação do módulo, não de uma
    /// votação: separá-lo de `abrir()` é o que impede a primeira governança a
    /// usar o contrato de pagar a conta de todas as outras.
    ///
    /// Sem ninguém chamar, instância e código arquivam em 7 dias (sonda 9) e
    /// voltam com `RestoreFootprintOp`. É degradação, não perda.
    pub fn manter(env: Env) -> u32 {
        let m = env.storage().max_ttl();
        env.storage().instance().extend_ttl(m - 1, m);
        m
    }

    // ===================== leitura =======================================

    pub fn proposta(env: Env, proposta: BytesN<32>) -> Option<Proposta> {
        env.storage().persistent().get(&Chave::Proposta(proposta))
    }

    /// Os acumuladores, para quem quiser recalcular a apuração por fora.
    pub fn acumulador(env: Env, proposta: BytesN<32>) -> Vec<Bls12381G1Affine> {
        let mut v = Vec::new(&env);
        if let Some(p) = Self::proposta(env.clone(), proposta.clone()) {
            for j in 0..p.opcoes {
                if let Some(a) = env
                    .storage()
                    .persistent()
                    .get::<_, Bls12381G1Affine>(&Chave::Acum(proposta.clone(), j))
                {
                    v.push_back(a);
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
