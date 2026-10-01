//! Matemática do Tessera, compartilhada entre contrato, cliente e verificador.
//!
//! Escrever provador e verificador a partir do mesmo código é o que impede o
//! formato de prova de divergir entre os dois — que é o bug mais caro possível
//! neste projeto (smoke B3).

pub mod acaso;
pub mod pedersen;
pub mod ponto;
