//! Os oito eventos do contrato, declarados como tipos.
//!
//! **Por que tipos e não `env.events().publish()`.** O `publish()` está
//! depreciado, e o problema dele aqui nunca foi o aviso do compilador: a tupla
//! que ele recebe não tem nome de campo nenhum, então o formato do evento — que
//! é interface congelada (§5), lida por `app/src/rede.ts` **por posição** —
//! vivia só no corpo da função e numa linha de comentário. Trocar dois campos
//! de lugar compilava.
//!
//! Aqui cada campo tem nome, o `#[topic]` diz o que é indexado, e a macro
//! inclui o evento na especificação do contrato, de onde um cliente gera os
//! tipos. O formato na rede é **o mesmo byte a byte** — `topics = [...]`
//! preserva o nome curto e `data_format = "vec"` preserva a tupla posicional —
//! e é isso que os três testes `o_formato_dos_eventos_*` conferem.

use soroban_sdk::{
    contractevent,
    crypto::bls12_381::{Bls12381Fr, Bls12381G1Affine},
    Address, Bytes, BytesN, Vec,
};

/// A votação existe. É por este evento que o dapp lista as votações.
#[contractevent(topics = ["abrir"], data_format = "vec")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Abrir {
    #[topic]
    pub proposta: BytesN<32>,
    pub n_perguntas: u32,
    pub conf_opcoes: u32,
    pub abre_em: u32,
    pub fecha_em: u32,
    pub limiar: u32,
    pub anel: bool,
    pub secoes: u32,
}

/// Uma cédula identificada. O nome de quem votou está no tópico — é o caminho
/// em que ele está, de propósito.
#[contractevent(topics = ["votar"], data_format = "vec")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Votar {
    #[topic]
    pub proposta: BytesN<32>,
    #[topic]
    pub votante: Address,
    pub compromissos: Vec<Bls12381G1Affine>,
    pub escolhas: Vec<u32>,
}

/// **O caderno.** Quem compareceu, e em que seção caiu.
#[contractevent(topics = ["comparec"], data_format = "vec")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comparec {
    #[topic]
    pub proposta: BytesN<32>,
    #[topic]
    pub votante: Address,
    pub secao: u32,
    pub tamanho: u32,
}

/// **A urna.** O único evento sem endereço nenhum, e é esse o ponto dele.
///
/// A ordem dos campos é interface congelada: `rede.ts` desempacota
/// `[imagem, compromissos, escolhas, cripto]` por posição, e quem apura
/// re-encadeia os compromissos nesta ordem contra `Cadeia(proposta, secao)`.
///
/// **A `secao` no tópico é a §11-N.** Sem ela, quem lê os eventos de uma
/// votação de várias seções não tem como dizer de que seção é cada cédula — e a
/// informação não está em lugar nenhum do ledger que o cliente alcance: o
/// criptograma não diz nada antes de ser decifrado e decifrar não revela a
/// seção; a assinatura em anel não revela qual anel assinou, que é o ponto
/// dela; e a cadeia é sequencial, então "descobrir" a atribuição seria testar
/// as partições possíveis. Com ela, a apuração de várias seções fecha.
///
/// Não revela nada novo: `secao_de` já é um getter, o anel de cada seção já é
/// público, e votar já exige declarar a seção na própria chamada.
#[contractevent(topics = ["anonimo"], data_format = "vec")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Anonimo {
    #[topic]
    pub proposta: BytesN<32>,
    #[topic]
    pub secao: u32,
    pub imagem: Bls12381G1Affine,
    pub compromissos: Vec<Bls12381G1Affine>,
    pub escolhas: Vec<u32>,
    pub cripto: Bytes,
}

/// Um membro da mesa endossou **estes números**, não "a apuração".
#[contractevent(topics = ["endosso"], data_format = "vec")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Endosso {
    #[topic]
    pub proposta: BytesN<32>,
    #[topic]
    pub membro: Address,
    pub digest: BytesN<32>,
    pub quantos: u32,
    pub limiar: u32,
}

/// A apuração da mesa fechou o limiar e publicou o resultado.
#[contractevent(topics = ["apurar"], data_format = "vec")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Apurar {
    #[topic]
    pub proposta: BytesN<32>,
    pub totais: Vec<u32>,
    pub aberturas: Vec<Bls12381Fr>,
    pub resultado: Vec<u32>,
}

/// A apuração de uma seção, sem mesa. `quantas` é o que a cadeia registrou, e
/// serve para quem lê os eventos saber quantas cédulas **deveria** ter achado.
///
/// **Sem placar**, desde T-026: o total daquela seção em claro revelava o voto
/// dos membros de uma seção unânime (DEC-017). O que a seção publica é uma
/// contagem; o número de voto sai de `publicar_resultado`.
#[contractevent(topics = ["apursec"], data_format = "vec")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Apursec {
    #[topic]
    pub proposta: BytesN<32>,
    #[topic]
    pub secao: u32,
    pub quantas_abertas: u32,
    pub quantas: u32,
}

/// A assinatura da baliza da rodada de abertura foi registrada e conferida.
/// Dado de valor único: a rodada crua, sem `Vec` em volta.
#[contractevent(topics = ["abertura"], data_format = "single-value")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Abertura {
    #[topic]
    pub proposta: BytesN<32>,
    pub rodada_abertura: u64,
}
