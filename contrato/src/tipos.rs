//! Tipos públicos do contrato: erros, chaves de armazenamento e provas.

use soroban_sdk::{
    contracterror, contracttype,
    crypto::bls12_381::{Bls12381Fr, Bls12381G1Affine},
    Address, BytesN, Vec,
};

/// Quórum de sigilo: mínimo de cédulas para que a apuração não entregue quem
/// votou. Ver PROTOCOLO §6.6.
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
/// Nenhum sistema eleitoral sério aceita isso. A prática é proteger a célula
/// pequena **antes** — agregando seções abaixo de um piso — e nunca recusar a
/// contagem depois; quando há nulidade, o remédio é eleição nova, não ausência
/// de resultado.
///
/// Removido `votar_publico`, não há partição a induzir: toda cédula é
/// sigilosa, e o único jeito de ficar abaixo de `TAU` é comparecimento baixo.
/// O remédio é o de sempre — estender o prazo ou refazer com um eleitorado que
/// caiba no sigilo.
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
    /// A v1 só aceita um-voto-por-pessoa. Ver PROTOCOLO §6.3: pesos públicos
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
    /// Menos de `TAU` cédulas confidenciais. PROTOCOLO §6.6: impor o limiar
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
    /// O comparecimento já fechou: `ledger >= abre_em`. Depois que a votação
    /// abre o anel está congelado, e aceitar mais um membro mudaria o conjunto
    /// debaixo de quem já votou.
    ComparecimentoEncerrado = 27,
    JaCompareceu = 28,
    /// A assinatura em anel não fecha, ou o anel apresentado não é o que foi
    /// registrado no comparecimento.
    AnelInvalido = 29,
    /// Esta imagem de chave já votou. É o voto duplo, detectado **sem** saber
    /// de quem é.
    ImagemJaUsada = 30,
    /// A proposta não é de anel, ou é de anel e a chamada identificada foi
    /// usada. Os dois modos não se misturam na mesma proposta.
    ModoErrado = 31,
    /// Seção fora da faixa `0..secoes`, ou `secoes = 0`.
    SecaoInvalida = 32,
    /// Numa votação aberta não há lista: o caminho de Merkle tem de vir vazio,
    /// e a seção quem decide é o contrato.
    VotacaoAberta = 33,
    /// A rodada da fechadura de tempo não é a que `fim_tempo` determina. É o
    /// portão que impede cifrar para uma rodada no passado — que abriria na
    /// hora — ou longe no futuro, que nunca abriria. Ver SPEC INV-22.
    RodadaNaoFecha = 34,
    /// A rodada da fechadura ainda não venceu: a chave que decifra as cédulas
    /// não existe no mundo. Recusar aqui é o que torna INV-18 incondicional.
    RelogioAindaNaoAbriu = 35,
    /// A lista de compromissos apresentada não re-encadeia no que as cédulas
    /// escreveram. É omissão ou invenção — ver SPEC INV-20.
    CadeiaNaoFecha = 36,
    /// A apuração apresentada abre **menos** cédulas que a já guardada. Só um
    /// conjunto estritamente maior substitui — ver SPEC INV-21.
    NaoMelhora = 37,
    /// O criptograma não tem 160 bytes por opção confidencial.
    CriptogramaMalFormado = 38,
    /// A assinatura apresentada não é a da rodada de abertura desta proposta.
    /// `e(σ, g₂) != e(H₁(rodada), P)` — é um relé mentindo, ou a rodada errada.
    BalizaNaoConfere = 39,
    /// A rodada de abertura ainda não venceu: a assinatura que decide as seções
    /// não existe no mundo, então ninguém pode comparecer.
    AberturaAindaNaoVenceu = 40,
    /// A assinatura da rodada de abertura não foi registrada. Qualquer pessoa
    /// pode registrá-la, e sem ela não há seção para conferir.
    BalizaNaoRegistrada = 41,
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
    ///
    /// **Conta cédulas, não presenças.** Quem conta presenças é `Caderno`.
    Comparecimento(BytesN<32>),
    Resultado(BytesN<32>),
    /// `Endosso(proposta, digest, membro)`: um membro da mesa endossou esta
    /// apuração exata. O digest é `sha256(totais ‖ aberturas)`, então endossar
    /// é endossar **estes números**, não "a apuração" em abstrato.
    Endosso(BytesN<32>, BytesN<32>, Address),
    /// Quantos membros já endossaram aquele digest.
    Endossos(BytesN<32>, BytesN<32>),
    /// **O caderno.** Um registro por membro que compareceu, identificado e
    /// público — é dele que sai a lista de quem faltou.
    ///
    /// Guarda a **seção** da pessoa, não um `bool`: na votação aberta quem
    /// decide a seção é o contrato, por ordem de chegada, e sem isto quem vota
    /// não teria como saber qual anel é o dela depois de trocar de navegador.
    Compareceu(BytesN<32>, Address),
    /// As chaves de anel de quem compareceu **naquela seção**, na ordem em que
    /// chegaram. Cresce durante o comparecimento e congela quando a votação
    /// abre. Uma entrada por seção: é o que faz o custo da cédula parar de
    /// depender do tamanho do eleitorado.
    Anel(BytesN<32>, u32),
    /// `sha256` do conjunto **daquela seção**, calculado uma vez quando a
    /// primeira cédula dela chega. A cédula traz a lista inteira e o contrato
    /// compara — 32 bytes de estado em vez de `n` pontos relidos a cada voto.
    DigestoAnel(BytesN<32>, u32),
    /// Quantas pessoas já compareceram. Entrada de tamanho fixo, de propósito:
    /// é ela que decide a seção na votação aberta, e um `u32` não muda de
    /// tamanho, então o footprint declarado continua valendo quando várias
    /// pessoas comparecem no mesmo instante.
    Caderno(BytesN<32>),
    /// **A cadeia de compromissos da seção**: `(sha256(anterior ‖
    /// compromissos), quantas cédulas)`, atualizada por cada cédula anônima.
    ///
    /// Trinta e seis bytes que **não crescem** — ao contrário de `Anel`, que
    /// cresce 96 B por pessoa e produziu a §11-D. É ela que prova, na apuração,
    /// que a lista apresentada é exatamente o conjunto de cédulas daquela
    /// seção: omitir uma muda o encadeamento, inventar uma também.
    Cadeia(BytesN<32>, u32),
    /// `(quantas cédulas abriram, totais confidenciais)` daquela seção. Só é
    /// substituído por uma apuração que abra **mais** cédulas (INV-21).
    ResultadoSecao(BytesN<32>, u32),
    /// Quantas seções já têm resultado. Quando bate com `secoes`, o placar
    /// existe.
    SecoesApuradas(BytesN<32>),
    /// A assinatura da baliza da rodada de abertura, **já conferida pelo
    /// contrato**, nos 96 bytes que o host lê. É dela que sai a seção.
    Abertura(BytesN<32>),
    /// Uma imagem de chave já usada. **Não é um endereço**: é `I = x·Hp`, que
    /// identifica a pessoa dentro desta proposta e em nenhuma outra.
    ImagemUsada(BytesN<32>, BytesN<32>),
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
    /// A rodada da baliza que destranca as cédulas.
    ///
    /// `fecha_em` é sequência de ledger e a fechadura precisa de relógio: a
    /// rodada vence num instante. Não existe campo de tempo separado porque
    /// seria redundante — o instante **é** `instante(rodada)`, e guardar os dois
    /// só criaria uma divergência possível. A apuração exige os dois portões: a
    /// sequência passou e a rodada venceu.
    ///
    /// `0` é proposta sem fechadura — a da mesa, que continua existindo.
    pub rodada: u64,
    /// A rodada da baliza que **abre o comparecimento**, e de cuja assinatura
    /// sai a seção de cada pessoa (DEC-012).
    ///
    /// Ela fica entre `abrir` e o começo do comparecimento, e é essa posição que
    /// faz o desenho funcionar: na hora de `abrir` a assinatura não existe, então
    /// quem organiza não consegue moer o que quer que seja até isolar alguém; e
    /// quando as pessoas comparecem ela já existe, então cada uma sabe a sua
    /// seção. Rodadas saem a cada 3 segundos. Ver §11-J.
    ///
    /// `0` é proposta sem seções derivadas da baliza.
    pub rodada_abertura: u64,
    /// **O caderno e a urna, separados.**
    ///
    /// Com `anel = true` a proposta tem duas fases: até `abre_em` as pessoas
    /// comparecem com nome e endereço, e o contrato monta o anel; depois disso
    /// as cédulas chegam de chaves efêmeras, com uma assinatura em anel que
    /// prova pertencimento sem dizer de quem é.
    ///
    /// É o desenho da urna: o caderno diz quem faltou — e voto obrigatório
    /// precisa disso —, a cédula não diz de quem é, e nada liga os dois.
    pub anel: bool,
    /// Quantas seções existem **agora**.
    ///
    /// Na fechada é fixo: sai da lista na abertura. Na aberta cresce sozinho —
    /// a seção enche até `limite_secao` e a próxima abre.
    ///
    /// A seção existe porque verificar um anel custa 10.822.850 instruções por
    /// membro: um anel de 30 usa 91,4% do teto de CPU de uma transação, e só
    /// uma cédula dessas cabe por ledger. Com seções, o custo por cédula para
    /// de depender do tamanho do eleitorado.
    ///
    /// O preço é o conjunto de anonimato: ele passa a ser a seção, não a
    /// votação inteira. O resultado continua único — o acumulador é por
    /// proposta e não sabe de que seção veio cada cédula.
    pub secoes: u32,
    /// Quantas pessoas cabem numa seção antes de a próxima abrir. `0` é uma
    /// seção só, sem limite.
    ///
    /// Só tem efeito na votação aberta: na fechada a lista é conhecida e a
    /// divisão sai dela na abertura, presa na folha de Merkle.
    pub limite_secao: u32,
}

/// Prova disjuntiva de Cramer–Damgård–Schoenmakers: `v ∈ {0,1}`.
/// 2 pontos + 4 escalares = 320 bytes. PROTOCOLO §6.1.
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
/// `D = (Σ C_j) − w·G = ρ·H`. Uma prova, não `m` provas. PROTOCOLO §6.2.
#[contracttype]
#[derive(Clone)]
pub struct ProvaSoma {
    pub a: Bls12381G1Affine,
    pub z: Bls12381Fr,
}
