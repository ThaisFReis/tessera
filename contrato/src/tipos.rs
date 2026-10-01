//! Tipos públicos do contrato: erros, chaves de armazenamento e provas.

use soroban_sdk::{
    contracterror, contracttype,
    crypto::bls12_381::{Bls12381Fr, Bls12381G1Affine},
    Address, BytesN, Vec,
};

/// Limiar mínimo de anonimato. Ver SPEC §6.6, o teorema da partição.
///
/// Nenhuma célula da partição induzida pelos campos públicos pode ser apurada
/// com menos de `TAU` cédulas confidenciais. Na v1 há duas células —
/// confidencial e pública — então a regra se reduz a: ou ninguém votou em
/// sigilo, ou pelo menos `TAU` votaram.
pub const TAU: u32 = 5;

/// Máximo de opções por proposta.
pub const MAX_OPCOES: u32 = 16;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Erro {
    PropostaJaExiste = 1,
    PropostaNaoExiste = 2,
    OpcoesForaDaFaixa = 3,
    LimiarInvalido = 4,
    PrazoNoPassado = 5,
    VotacaoEncerrada = 6,
    VotacaoAindaAberta = 7,
    JaVotou = 8,
    NaoEstaNaListaDeAptos = 9,
    /// A v1 só aceita um-voto-por-pessoa. Ver SPEC §6.3: pesos públicos
    /// distintos são quebrados por subconjunto-soma, e o contrato **recusa**
    /// essa configuração em vez de documentá-la como cuidado.
    PesoNaoUnitario = 10,
    ProvaBinariaInvalida = 11,
    ProvaDeSomaInvalida = 12,
    PontoForaDoSubgrupo = 13,
    /// A mesa publicou `(T, R)` que não abre o acumulador. É a mentira que o
    /// compromisso de Pedersen torna detectável por qualquer pessoa.
    AberturaNaoFecha = 14,
    JaApurada = 15,
    MesaAbaixoDoLimiar = 16,
    NaoEMembroDaMesa = 17,
    MembroRepetido = 18,
    /// Menos de `TAU` cédulas confidenciais. SPEC §6.6: impor o limiar
    /// converte uma quebra de privacidade numa falha de liveness.
    AnonimatoInsuficiente = 19,
    ArgumentoMalFormado = 20,
    EscolhaForaDoBinario = 21,
    SomaDiferenteDoPeso = 22,
    TotalDiferenteDoComparecimento = 23,
}

#[contracttype]
#[derive(Clone)]
pub enum Chave {
    /// Metadados da proposta. TTL estendido ao teto da rede em `abrir()`.
    Proposta(BytesN<32>),
    /// `A_j`, a soma dos compromissos confidenciais da opção `j`. TTL ao teto.
    Acum(BytesN<32>, u32),
    /// Uma entrada por votante. TTL padrão — pode arquivar depois de encerrar,
    /// e estender todas custaria ~2.070 XLM numa assembleia de 10.000 (A1).
    Votou(BytesN<32>, Address),
    /// Total em claro da opção `j`, vindo das cédulas públicas.
    TotalPublico(BytesN<32>, u32),
    /// `(confidenciais, públicas)`. É o que a regra de `TAU` consulta.
    Comparecimento(BytesN<32>),
    Resultado(BytesN<32>),
}

/// Chave de instância do segundo gerador.
#[contracttype]
#[derive(Clone)]
pub enum Instancia {
    GeradorH,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposta {
    pub opcoes: u32,
    pub raiz_aptos: BytesN<32>,
    pub mesa: Vec<Address>,
    pub limiar: u32,
    /// Sequência de ledger a partir da qual não se vota mais.
    pub fecha_em: u32,
}

/// Prova disjuntiva de Cramer–Damgård–Schoenmakers: `v ∈ {0,1}`.
/// 2 pontos + 4 escalares = 320 bytes. SPEC §6.1.
#[contracttype]
#[derive(Clone)]
pub struct ProvaCds {
    pub a0: Bls12381G1Affine,
    pub a1: Bls12381G1Affine,
    pub e0: Bls12381Fr,
    pub z0: Bls12381Fr,
    pub e1: Bls12381Fr,
    pub z1: Bls12381Fr,
}

/// Schnorr em base `H`, provando conhecimento de `ρ = Σ r_j` com
/// `D = (Σ C_j) − w·G = ρ·H`. Uma prova, não `m` provas. SPEC §6.2.
#[contracttype]
#[derive(Clone)]
pub struct ProvaSoma {
    pub a: Bls12381G1Affine,
    pub z: Bls12381Fr,
}
