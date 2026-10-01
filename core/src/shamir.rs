//! Compartilhamento de Shamir sobre `Fr`, para a mesa apuradora `k`-de-`N`.
//!
//! **O problema que isto resolve.** Se a mesa for uma entidade só, ela recebe
//! `r_{i,j}` e, com o `C_{i,j}` que é público, calcula `C − r·H = v·G` e lê o
//! voto individual. Uma mesa única **vê todos os votos** — e a mesa é
//! tipicamente indicada por quem já tem poder na governança. Ver SPEC §4.3.
//!
//! **A propriedade que faz funcionar.** Shamir é aditivamente homomórfico *nas
//! shares*: somar as shares de índice `l` de vários segredos dá uma share de
//! índice `l` da soma. Então cada membro da mesa soma localmente
//!
//! ```text
//! S_{j,l} = Σ_i s_{i,j,l}
//! ```
//!
//! e `k` membros reconstroem `R_j = Σ_i r_{i,j}` — que é exatamente o número
//! que a apuração precisa publicar — **sem que nenhum `r_{i,j}` individual
//! jamais se junte em lugar nenhum.** A informação necessária para ler um voto
//! não existe reunida em nenhum lugar do sistema.
//!
//! **O que resta:** `k` membros em conluio que tenham guardado as shares
//! *brutas*, antes de somar, conseguem. Daí N3 ser procedimental.

use crate::pedersen::acaso_fr;
use ark_ff::{Field, Zero};
use ark_bls12_381::Fr;

#[derive(Debug, PartialEq)]
pub enum Erro {
    LimiarInvalido { k: usize, n: usize },
    IndiceZero,
    IndicesRepetidos,
    PoucasShares { tem: usize, precisa: usize },
    SemAleatoriedade,
    ComprimentosDiferentes,
}

/// Uma share: `(índice do membro, f(índice))`.
///
/// O índice é 1-based e nunca zero, porque `f(0)` **é** o segredo.
#[derive(Clone, Debug, PartialEq)]
pub struct Share {
    pub membro: u32,
    pub valor: Fr,
}

/// Divide `segredo` em `n` shares com limiar `k`.
///
/// `f(x) = segredo + a₁x + … + a_{k-1}x^{k-1}`, coeficientes uniformes em `Fr`.
/// Share do membro `l` é `f(l)`, para `l` em `1..=n`.
pub fn dividir(segredo: &Fr, k: usize, n: usize) -> Result<Vec<Share>, Erro> {
    if k == 0 || k > n || n == 0 {
        return Err(Erro::LimiarInvalido { k, n });
    }
    // coeficientes: [segredo, a₁, …, a_{k-1}]
    let mut coef = Vec::with_capacity(k);
    coef.push(*segredo);
    for _ in 1..k {
        coef.push(acaso_fr().map_err(|_| Erro::SemAleatoriedade)?);
    }
    Ok((1..=n as u32)
        .map(|l| Share {
            membro: l,
            valor: avaliar(&coef, &Fr::from(l as u64)),
        })
        .collect())
}

/// `f(x)` por Horner.
fn avaliar(coef: &[Fr], x: &Fr) -> Fr {
    coef.iter().rev().fold(Fr::zero(), |acc, c| acc * x + c)
}

/// Reconstrói `f(0)` por interpolação de Lagrange.
///
/// Exige pelo menos `k` shares de índices distintos e não-nulos. Shares a mais
/// são aceitas: o polinômio é o mesmo.
pub fn reconstruir(shares: &[Share], k: usize) -> Result<Fr, Erro> {
    if shares.len() < k {
        return Err(Erro::PoucasShares { tem: shares.len(), precisa: k });
    }
    let usar = &shares[..k];
    for (i, s) in usar.iter().enumerate() {
        if s.membro == 0 {
            return Err(Erro::IndiceZero);
        }
        if usar[..i].iter().any(|o| o.membro == s.membro) {
            return Err(Erro::IndicesRepetidos);
        }
    }

    // s = Σ_j y_j · Π_{m≠j} x_m / (x_m − x_j)
    let mut soma = Fr::zero();
    for (j, sj) in usar.iter().enumerate() {
        let xj = Fr::from(sj.membro as u64);
        let mut num = Fr::from(1u64);
        let mut den = Fr::from(1u64);
        for (m, sm) in usar.iter().enumerate() {
            if m == j {
                continue;
            }
            let xm = Fr::from(sm.membro as u64);
            num *= xm;
            den *= xm - xj;
        }
        // den nunca é zero: índices distintos, e Fr é corpo.
        soma += sj.valor * num * den.inverse().expect("indices distintos => den != 0");
    }
    Ok(soma)
}

/// **O coração da mesa `k`-de-`N`.**
///
/// Cada membro recebe uma share de cada `r_i` e soma as suas localmente. Esta
/// função é o que o membro `l` roda no seu próprio computador — e o resultado
/// é a única coisa que ele devolve. Os `s_{i,l}` brutos são destruídos aqui.
pub fn somar_shares(minhas: &[Share]) -> Result<Share, Erro> {
    let membro = minhas.first().ok_or(Erro::PoucasShares { tem: 0, precisa: 1 })?.membro;
    if minhas.iter().any(|s| s.membro != membro) {
        return Err(Erro::ComprimentosDiferentes);
    }
    Ok(Share {
        membro,
        valor: minhas.iter().fold(Fr::zero(), |acc, s| acc + s.valor),
    })
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::pedersen::escalar;

    /// **Smoke B5.** Cinco votantes, mesa de 5, limiar 3.
    ///
    /// Cada votante divide o seu `r_i`. Cada membro soma as 5 shares que
    /// recebeu. Três membros reconstroem `Σr_i` — e nenhum `r_i` individual
    /// jamais foi reunido.
    #[test]
    fn mesa_soma_sem_nunca_reunir_um_voto() {
        let rs: Vec<Fr> = (0..5).map(|_| acaso_fr().unwrap()).collect();
        let (k, n) = (3usize, 5usize);

        // cada votante divide o seu r_i e manda uma share a cada membro
        let por_votante: Vec<Vec<Share>> =
            rs.iter().map(|r| dividir(r, k, n).unwrap()).collect();

        // cada membro l soma o que recebeu, e isso é tudo que ele devolve
        let parciais: Vec<Share> = (0..n)
            .map(|l| {
                let minhas: Vec<Share> =
                    por_votante.iter().map(|v| v[l].clone()).collect();
                somar_shares(&minhas).unwrap()
            })
            .collect();

        let esperado = rs.iter().fold(Fr::zero(), |a, r| a + r);

        // três membros quaisquer reconstroem a SOMA
        for trio in [[0, 1, 2], [0, 2, 4], [1, 3, 4], [2, 3, 4]] {
            let sub: Vec<Share> = trio.iter().map(|&i| parciais[i].clone()).collect();
            assert_eq!(
                reconstruir(&sub, k).unwrap(),
                esperado,
                "trio {:?} nao reconstruiu a soma",
                trio
            );
        }
    }

    /// Dois membros não reconstroem. E o motivo é mais forte do que "dá outro
    /// número": dois pontos são consistentes com **todo** segredo possível.
    #[test]
    fn dois_membros_nao_sabem_nada() {
        let segredo = acaso_fr().unwrap();
        let shares = dividir(&segredo, 3, 5).unwrap();
        let dois = vec![shares[0].clone(), shares[3].clone()];

        // o que der, nao e o segredo
        assert_ne!(reconstruir(&dois, 2).unwrap(), segredo);
        assert_eq!(
            reconstruir(&dois, 3),
            Err(Erro::PoucasShares { tem: 2, precisa: 3 })
        );

        // o argumento de verdade: para QUALQUER candidato, existe um polinomio
        // de grau 2 que passa pelos dois pontos e tem f(0) = candidato. Logo
        // os dois pontos nao excluem nenhum segredo.
        for candidato in [escalar(0), escalar(1), escalar(999), segredo] {
            let tres = vec![
                Share { membro: 0, valor: candidato }, // f(0) = candidato
                dois[0].clone(),
                dois[1].clone(),
            ];
            // interpola os 3 e confere que o polinomio bate nos dois pontos
            // conhecidos — por construcao bate, e e esse o ponto: nada no que
            // dois membros tem contradiz candidato algum.
            let f0 = interpolar_em(&tres, &Fr::zero());
            assert_eq!(f0, candidato, "candidato {:?} seria excluido", candidato);
        }
    }

    /// Lagrange em um `x` qualquer, só para o teste acima. O código de produção
    /// só precisa de `x = 0`.
    fn interpolar_em(pts: &[Share], x: &Fr) -> Fr {
        let mut soma = Fr::zero();
        for (j, sj) in pts.iter().enumerate() {
            let xj = Fr::from(sj.membro as u64);
            let (mut num, mut den) = (Fr::from(1u64), Fr::from(1u64));
            for (m, sm) in pts.iter().enumerate() {
                if m == j { continue; }
                let xm = Fr::from(sm.membro as u64);
                num *= *x - xm;
                den *= xj - xm;
            }
            soma += sj.valor * num * den.inverse().unwrap();
        }
        soma
    }

    /// Homomorfismo aditivo nas shares, isolado: share(a) + share(b) é uma
    /// share de a+b, no mesmo índice. É a propriedade inteira da mesa.
    #[test]
    fn shares_somam_como_os_segredos() {
        let (a, b) = (escalar(12345), escalar(67890));
        let (sa, sb) = (dividir(&a, 2, 3).unwrap(), dividir(&b, 2, 3).unwrap());
        let soma: Vec<Share> = (0..3)
            .map(|l| somar_shares(&[sa[l].clone(), sb[l].clone()]).unwrap())
            .collect();
        assert_eq!(reconstruir(&soma[..2], 2).unwrap(), a + b);
        assert_eq!(reconstruir(&soma[1..], 2).unwrap(), a + b);
    }

    /// Limiar 1 é degenerado mas legal: toda share é o segredo.
    #[test]
    fn limiar_um_e_n_valem() {
        let s = escalar(42);
        for sh in dividir(&s, 1, 4).unwrap() {
            assert_eq!(sh.valor, s);
        }
        let shares = dividir(&s, 4, 4).unwrap();
        assert_eq!(reconstruir(&shares, 4).unwrap(), s);
        assert_ne!(reconstruir(&shares[..3], 3).unwrap(), s);
    }

    #[test]
    fn recusa_configuracao_impossivel() {
        let s = escalar(1);
        assert_eq!(dividir(&s, 4, 3), Err(Erro::LimiarInvalido { k: 4, n: 3 }));
        assert_eq!(dividir(&s, 0, 3), Err(Erro::LimiarInvalido { k: 0, n: 3 }));
    }

    #[test]
    fn recusa_shares_repetidas_e_indice_zero() {
        let shares = dividir(&escalar(7), 2, 3).unwrap();
        let repetidas = vec![shares[1].clone(), shares[1].clone()];
        assert_eq!(reconstruir(&repetidas, 2), Err(Erro::IndicesRepetidos));

        let com_zero = vec![Share { membro: 0, valor: escalar(1) }, shares[0].clone()];
        assert_eq!(reconstruir(&com_zero, 2), Err(Erro::IndiceZero));
    }

    /// Uma share adulterada por um membro malicioso envenena a reconstrução.
    /// Shamir simples não detecta isso — e é por isso que a mesa publica
    /// `(T, R)` e o **contrato confere** `A == T·G + R·H`. A detecção não vem
    /// daqui; vem do compromisso. Este teste trava essa expectativa.
    #[test]
    fn membro_mentindo_quebra_a_soma_e_quem_pega_e_o_compromisso() {
        let segredo = escalar(1000);
        let mut shares = dividir(&segredo, 3, 5).unwrap();
        shares[1].valor += escalar(1);
        assert_ne!(reconstruir(&shares[..3], 3).unwrap(), segredo);
    }
}
