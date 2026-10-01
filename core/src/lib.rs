//! Matemática do Tessera, compartilhada entre contrato, cliente e verificador.
//!
//! Escrever provador e verificador a partir do mesmo código é o que impede o
//! formato de prova de divergir entre os dois — que é o bug mais caro possível
//! neste projeto (smoke B3).

pub mod acaso;
pub mod cds;
pub mod merkle;
pub mod pedersen;
pub mod ponto;
pub mod shamir;
pub mod soma;

/// Os tipos do arkworks que o `core` expõe, para quem o consome não precisar
/// repetir a dependência — e, mais importante, não conseguir repetir numa
/// **versão diferente**. O host do Soroban usa arkworks; divergir de versão
/// aqui seria exatamente o bug que este crate existe para impedir.
pub mod ark {
    pub use ark_bls12_381::{Fr, G1Affine};
}
