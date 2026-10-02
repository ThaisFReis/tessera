//! Tipos públicos do contrato: erros, chaves de armazenamento e provas.

use soroban_sdk::{
    contracterror, contracttype,
    crypto::bls12_381::{Bls12381Fr, Bls12381G1Affine},
    Address, BytesN, Vec,
};

/// Quórum de sigilo: mínimo de cédulas para que a apuração não entregue quem
/// votou. Ver SPEC §6.6.
///
/// Com poucas cédulas o total determina os votos por subtração — no limite,
/// uma cédula só *é* o voto daquela pessoa. O sigilo dos compromissos é
/// perfeito **e irrelevante**: quem ataca usa aritmética, não criptanálise.
///
/// **Isto é quórum, declarado na abertura — não uma recusa surpresa.** A
/// diferença importa, e custou um redesenho: enquanto existia `votar_publico`,
/// qualquer um podia abrir o próprio voto e encolher o conjunto sigiloso de
/// fora, e três pessoas bastavam para vetar a assembleia inteira. Recusar o
/// resultado correto virava negação de serviço contra a eleição.
///
/// Nenhum sistema eleitoral sério aceita isso. O Brasil protege a célula
/// pequena **antes**, agregando seções com menos de 50 eleitores, e nunca
/// recusa a contagem depois (TSE, Res. 23.669/2021); quando há nulidade, o
/// remédio é eleição nova (CE art. 224), não ausência de resultado.
///
/// Removido `votar_publico`, não há partição a induzir: toda cédula é
/// sigilosa, e o único jeito de ficar abaixo de `TAU` é comparecimento baixo.
/// O remédio é o mesmo do Brasil — estender o prazo ou refazer com um
/// eleitorado que caiba no sigilo.
///
/// Perguntas públicas não mexem nisso: são do estatuto, iguais para todos, e
/// não distinguem um eleitor de outro.
pub const TAU: u32 = 5;

/// Máximo de perguntas numa cédula.
pub const MAX_PERGUNTAS: u32 = 8;

/// Máximo de opções **confidenciais somadas** na cédula inteira.
///
/// É o que limita a CPU, e o limite é medido, não arbitrado: `votar()` custa
/// 9.805.000 fixos mais 13.501.500 por opção confidencial, perfeitamente
/// linear. Com 16 opções a cédula consome 225.920.355 — 56,5% do teto de uma
/// transação, com 43% de folga para o resto do envelope.
///
/// As opções públicas não entram nesta conta: uma cédula pública inteira custa
/// 350.372, três ordens de grandeza abaixo de uma única disjuntiva.
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
    MembroJaEndossou = 24,
    /// Nenhuma pergunta, perguntas demais, ou opções confidenciais somadas
    /// acima de `MAX_OPCOES`.
    PerguntasForaDaFaixa = 25,
    /// A janela ainda não começou: `ledger < abre_em`.
    VotacaoAindaNaoComecou = 26,
}

#[contracttype]
#[derive(Clone)]
pub enum Chave {
    /// Metadados da proposta. TTL estendido ao teto da rede em `abrir()`.
    Proposta(BytesN<32>),
    /// `A_{q,j}`, a soma dos compromissos confidenciais da opção `j` da
    /// pergunta `q`. Só existe para perguntas sigilosas. TTL ao teto.
    Acum(BytesN<32>, u32, u32),
    /// Uma entrada por votante. TTL padrão — pode arquivar depois de encerrar,
    /// e estender todas custaria ~2.070 XLM numa assembleia de 10.000 (A1).
    ///
    /// **Uma só por cédula, não uma por pergunta.** É o que torna a cédula
    /// mista atômica: ou a pessoa respondeu a cédula inteira, ou não votou.
    Votou(BytesN<32>, Address),
    /// Total em claro da opção `j` da pergunta `q`. Recebe de duas origens: as
    /// respostas em claro de quem votou por `votar()` nas perguntas públicas,
    /// e a cédula inteira de quem abriu o voto por `votar_publico()`.
    TotalPublico(BytesN<32>, u32, u32),
    /// `(confidenciais, públicas)`. É o que a regra de `TAU` consulta.
    Comparecimento(BytesN<32>),
    Resultado(BytesN<32>),
    /// `Endosso(proposta, digest, membro)`: um membro da mesa endossou esta
    /// apuração exata. O digest é `sha256(totais ‖ aberturas)`, então endossar
    /// é endossar **estes números**, não "a apuração" em abstrato.
    Endosso(BytesN<32>, BytesN<32>, Address),
    /// Quantos membros já endossaram aquele digest.
    Endossos(BytesN<32>, BytesN<32>),
}

/// Chave de instância do segundo gerador.
#[contracttype]
#[derive(Clone)]
pub enum Instancia {
    GeradorH,
}

/// Uma pergunta da cédula.
///
/// **A confidencialidade é da pergunta, fixada em `abrir()` — nunca escolhida
/// pelo eleitor por pergunta.** Se cada pessoa escolhesse onde se esconder, a
/// escolha de se esconder seria ela própria pública, e numa assembleia pequena
/// "quem pediu sigilo na pergunta 2" é uma lista curta o bastante para ser uma
/// acusação. Fixando na proposta, toda cédula tem a mesma forma e não há nada
/// a inferir da forma.
///
/// O eleitor mantém uma escolha, mas ela é da cédula inteira: votar em sigilo
/// (`votar`) ou abrir o voto todo (`votar_publico`). É a revelação voluntária,
/// e é o τ que protege quem ficou no grupo residual.
#[contracttype]
#[derive(Clone)]
pub struct Pergunta {
    pub opcoes: u32,
    pub confidencial: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposta {
    /// A cédula, em ordem. Uma cédula toda confidencial é o caso em que todas
    /// as perguntas têm `confidencial: true` — não é outro caminho de código.
    pub perguntas: Vec<Pergunta>,
    pub raiz_aptos: BytesN<32>,
    pub mesa: Vec<Address>,
    pub limiar: u32,
    /// Sequência de ledger a partir da qual se pode votar.
    ///
    /// Abertura imediata é `abre_em <= ledger atual` — não é outro caminho de
    /// código, é o mesmo portão com a janela já começada.
    pub abre_em: u32,
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
