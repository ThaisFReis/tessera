//! Compromisso de Pedersen sobre G1, e a abertura do agregado.
//!
//! `C = v·G + r·H`, com `H` de log discreto desconhecido em relação a `G`.
//!
//! **Perfeitamente ocultante:** para todo `v'` existe exatamente um `r'` com
//! `v'·G + r'·H = C`. Logo `C` é uniforme em G1 e independente de `v`, e não
//! há o que decifrar — com qualquer poder computacional, para sempre.
//!
//! **Computacionalmente vinculante:** abrir o mesmo `C` para dois valores
//! distintos revela `log_G(H)`. É o que impede a mesa de mentir no total.
//!
//! A assimetria é deliberada e é o oposto da escolha de um esquema de cifra.
//! Se o log discreto cair amanhã, uma mesa maliciosa passa a conseguir
//! falsificar um total — detectável, contestável, reparável. Se o sigilo fosse
//! computacional, a queda revelaria todos os votos já dados, retroativamente e
//! sem reparo. Trocamos um risco irreversível por um reversível.

use crate::acaso;
use crate::ponto;
use ark_bls12_381::{Fr, G1Affine, G1Projective};
use ark_ec::{AffineRepr, PrimeGroup};
use ark_ff::PrimeField;

/// O gerador canônico de G1.
pub fn gerador() -> G1Affine {
    G1Projective::generator().into()
}

/// Escalar a partir de um inteiro pequeno (um voto, um total).
pub fn escalar(v: u64) -> Fr {
    Fr::from(v)
}

/// Escalar uniforme em `[0, r)`, do CSPRNG do sistema.
///
/// É o `r` do compromisso. Ver `acaso.rs`: a uniformidade sobre TODO o corpo
/// é o que sustenta a ocultação perfeita, e por isso o sorteio é por rejeição.
pub fn acaso_fr() -> Result<Fr, acaso::Erro> {
    Ok(Fr::from_be_bytes_mod_order(&acaso::escalar_aleatorio()?))
}

/// `C = v·G + r·H`.
pub fn comprometer(g: &G1Affine, h: &G1Affine, v: &Fr, r: &Fr) -> G1Affine {
    ((*g * v) + (*h * r)).into()
}

/// Soma de compromissos — o que o acumulador `Acum(id, j)` faz a cada voto.
///
/// Aditivamente homomórfico: a soma dos compromissos é o compromisso da soma,
/// com a soma das aleatoriedades. É isto que permite apurar somando pontos,
/// sem abrir nenhum voto.
pub fn agregar(cs: &[G1Affine]) -> G1Affine {
    cs.iter()
        .fold(G1Projective::from(G1Affine::identity()), |acc, c| acc + c)
        .into()
}

/// Confere a abertura do agregado: `A == total·G + soma_r·H`.
///
/// A mesa publica `(total, soma_r)`. Revelar `soma_r` não revela nenhum `r_i`
/// individual: é a soma de n valores uniformes, e conhecer a soma de n
/// incógnitas não determina nenhuma delas.
pub fn verifica_agregado(
    a: &G1Affine,
    g: &G1Affine,
    h: &G1Affine,
    total: u64,
    soma_r: &Fr,
) -> bool {
    *a == comprometer(g, h, &escalar(total), soma_r)
}

/// Serializa um escalar como DECIMAL, que é o que a `stellar contract invoke`
/// aceita para `Bls12381Fr`. Passar hex falha — e um hex que por acaso só
/// tenha dígitos é aceito em silêncio como o decimal errado.
pub fn fr_para_decimal(f: &Fr) -> String {
    f.into_bigint().to_string()
}

/// Conveniência para a CLI: ponto em hex minúsculo.
pub fn para_hex(p: &G1Affine) -> String {
    ponto::para_hex(p)
}

#[cfg(test)]
mod testes {
    use super::*;

    const H_HEX: &str = "1462b4b57a7d01e598685e913608bab6990de8cce1c705642c19a6b9ed660fc589758bdebb20c2bdde65a25b35b12206198669dff273abe7b9e6a0b3e636f8a8a41f9c81f6a79d308ec0fe57ef8a073c90e444002ba59807f987cce67d72eccf";
    const A_HEX: &str = "042dbf56c0dcf1810bfe336c6e0079363d69bebe00c686d05405db39e56342bdde8edc1643cd462ec67fd57ab1fc314e12df9b00f64aae5deff4a171836c68a5785ef931e6559a481455d2659f6003d4c707141c89b605b7b2dc3bda48fa4e67";

    /// **O teste que importa (smoke B3 para o caminho Pedersen).**
    ///
    /// Reproduz, em Rust nativo, exatamente a votação que a sonda 7 executou
    /// na testnet: votos 1,0,1,1,0 com acasos 101..505. Se o agregado
    /// serializar byte a byte igual ao que o contrato produziu, então o
    /// provador off-chain e o verificador on-chain concordam — que é onde a
    /// maioria dos projetos de cripto quebra, e sempre por ordem de bytes,
    /// DST divergente ou redução módulo r.
    #[test]
    fn agregado_bate_byte_a_byte_com_a_testnet() {
        let g = gerador();
        let h = ponto::de_hex(H_HEX).expect("H da testnet nao desserializa");

        let votos: [u64; 5] = [1, 0, 1, 1, 0];
        let acasos: [u64; 5] = [101, 202, 303, 404, 505];

        let cs: Vec<G1Affine> = votos
            .iter()
            .zip(acasos.iter())
            .map(|(v, r)| comprometer(&g, &h, &escalar(*v), &escalar(*r)))
            .collect();

        let a = agregar(&cs);
        assert_eq!(
            para_hex(&a),
            A_HEX,
            "o agregado nativo DIVERGE do que o contrato produziu na testnet"
        );

        // e a abertura que a mesa publicou fecha
        let total: u64 = votos.iter().sum();
        let soma_r: u64 = acasos.iter().sum();
        assert_eq!(total, 3);
        assert_eq!(soma_r, 1515);
        assert!(verifica_agregado(&a, &g, &h, total, &escalar(soma_r)));
    }

    /// Mesa mentindo: os mesmos negativos que o contrato recusou na testnet.
    #[test]
    fn recusa_total_falso() {
        let g = gerador();
        let h = ponto::de_hex(H_HEX).unwrap();
        let a = ponto::de_hex(A_HEX).unwrap();
        let r = escalar(1515);

        assert!(verifica_agregado(&a, &g, &h, 3, &r));
        assert!(!verifica_agregado(&a, &g, &h, 4, &r));
        assert!(!verifica_agregado(&a, &g, &h, 2, &r));
        assert!(!verifica_agregado(&a, &g, &h, 3, &escalar(1516)));
    }

    #[test]
    fn homomorfismo() {
        let g = gerador();
        let h = ponto::de_hex(H_HEX).unwrap();
        let (r1, r2) = (escalar(7777), escalar(8888));
        let soma = agregar(&[
            comprometer(&g, &h, &escalar(1), &r1),
            comprometer(&g, &h, &escalar(1), &r2),
        ]);
        assert_eq!(soma, comprometer(&g, &h, &escalar(2), &(r1 + r2)));
    }

    /// A ocultação perfeita é um teorema (SPEC §3.2), não algo que um teste
    /// unitário prove. O que dá para fazer em código é a checagem empírica
    /// grosseira: compromissos a 0 e a 1, com `r` independente, têm de ser
    /// indistinguíveis por qualquer estatística simples dos bytes.
    ///
    /// Isto não prova o teorema. Pega a implementação que o violaria — `r`
    /// com entropia curta, viés de redução, ou um sorteio que depende de `v`.
    #[test]
    fn compromissos_a_zero_e_a_um_sao_indistinguiveis() {
        let g = gerador();
        let h = ponto::de_hex(H_HEX).unwrap();
        let n = 1500;

        // NAO use o byte 0: `x` e elemento de Fq, cujo modulo comeca em 0x1a,
        // entao o byte alto so assume 27 valores (0x00..0x1a) por construcao.
        // Um byte do meio e que e uniforme sobre 0..255.
        const BYTE: usize = 20;

        let amostra = |v: u64| -> (Vec<u8>, u64) {
            let mut meio = Vec::with_capacity(n);
            let mut bits: u64 = 0;
            for _ in 0..n {
                let c = comprometer(&g, &h, &escalar(v), &acaso_fr().unwrap());
                let b = ponto::serializar(&c);
                meio.push(b[BYTE]);
                bits += b.iter().map(|x| x.count_ones() as u64).sum::<u64>();
            }
            (meio, bits)
        };

        let (b0, bits0) = amostra(0);
        let (b1, bits1) = amostra(1);

        // nenhum dos dois pode ser degenerado
        for (rotulo, v) in [("v=0", &b0), ("v=1", &b1)] {
            let distintos: std::collections::HashSet<_> = v.iter().collect();
            assert!(
                distintos.len() > 200,
                "{}: byte do meio so assumiu {} valores em {} amostras",
                rotulo,
                distintos.len(),
                n
            );
        }

        // e os dois têm de ter a mesma cara
        let desvio = (bits0 as f64 - bits1 as f64).abs() / bits0 as f64;
        assert!(
            desvio < 0.01,
            "densidade de bits difere {:.2}% entre v=0 e v=1",
            desvio * 100.0
        );
    }

    /// A aritmética do compromisso fecha: `C - v·G - r·H` é a identidade.
    #[test]
    fn compromisso_fecha_na_aritmetica() {
        let g = gerador();
        let h = ponto::de_hex(H_HEX).unwrap();
        let (v, r) = (escalar(1), acaso_fr().unwrap());
        let c: G1Projective = comprometer(&g, &h, &v, &r).into();
        let resto: G1Affine = (c - g * v - h * r).into();
        assert_eq!(resto, G1Affine::identity());
    }

    /// `Fr` vai para a CLI em decimal. Travar isso em teste porque a armadilha
    /// já mordeu uma vez: hex com só dígitos é aceito como o decimal errado.
    #[test]
    fn escalar_serializa_em_decimal() {
        assert_eq!(fr_para_decimal(&escalar(1515)), "1515");
        assert_eq!(fr_para_decimal(&escalar(101)), "101");
    }
}

/// Escalar → 32 bytes big-endian canônicos, que é como `Bls12381Fr::from_bytes`
/// lê. O contrato **não reduz** módulo `r` na leitura, então os bytes têm de
/// já ser canônicos — e são, porque `acaso.rs` sorteia por rejeição.
pub fn fr_para_bytes_be(f: &Fr) -> [u8; 32] {
    use ark_ff::BigInteger;
    let v = f.into_bigint().to_bytes_be();
    let mut b = [0u8; 32];
    b[32 - v.len()..].copy_from_slice(&v);
    b
}
