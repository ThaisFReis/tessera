//! Árvore de Merkle da lista de aptos, e o caminho que quem vota apresenta.
//!
//! A folha é `H(endereço ‖ peso)`. A governança integradora monta a árvore na
//! data de corte e passa só a **raiz** para `abrir()`. Quem vota apresenta o
//! caminho; o contrato confere e usa **o peso da folha, nunca o peso informado
//! na chamada** (SPEC §6.4).
//!
//! Isto revela *quem* votou — intencional, é N1, a governança precisa de
//! quórum. Não revela nada sobre *em quê*.
//!
//! ## Três detalhes que são vulnerabilidade quando esquecidos
//!
//! 1. **Separação de domínio.** Folha e nó interno têm prefixos distintos
//!    (`0x00` e `0x01`). Sem isso, uma folha de 64 bytes bem escolhida pode ser
//!    apresentada como se fosse um nó interno, e alguém prova pertencimento de
//!    quem não está na lista.
//! 2. **Preenchimento, não duplicação.** Com número ímpar de folhas, a prática
//!    comum é duplicar a última. Isso dá **duas** provas válidas para o mesmo
//!    conjunto e muda a raiz de forma ambígua. Aqui a árvore é preenchida até a
//!    potência de dois com uma folha-vazia de domínio próprio (`0x02`).
//! 3. **Posição vem do índice.** O lado de cada irmão é o bit correspondente do
//!    índice, não um booleano que quem prova escolhe. Um caminho não pode ser
//!    reordenado.

use sha2::{Digest, Sha256};

pub type Hash = [u8; 32];

const DOM_FOLHA: u8 = 0x00;
const DOM_NO: u8 = 0x01;
const DOM_VAZIO: u8 = 0x02;

#[derive(Debug, PartialEq)]
pub enum Erro {
    ListaVazia,
    IndiceForaDaLista { indice: usize, total: usize },
    ProfundidadeErrada { tem: usize, precisa: usize },
}

/// Uma linha da lista de aptos na data de corte.
#[derive(Clone, Debug, PartialEq)]
pub struct Apto {
    /// Chave pública bruta da conta, ou id de contrato. 32 bytes.
    pub endereco: [u8; 32],
    /// Peso, ou identificador de faixa. Conferido contra a folha, nunca
    /// aceito do que quem vota afirma.
    pub peso: u32,
}

/// `H(0x00 ‖ endereço ‖ peso_be)`.
pub fn folha(a: &Apto) -> Hash {
    let mut h = Sha256::new();
    h.update([DOM_FOLHA]);
    h.update(a.endereco);
    h.update(a.peso.to_be_bytes());
    h.finalize().into()
}

/// `H(0x01 ‖ esquerda ‖ direita)`.
pub fn no(esq: &Hash, dir: &Hash) -> Hash {
    let mut h = Sha256::new();
    h.update([DOM_NO]);
    h.update(esq);
    h.update(dir);
    h.finalize().into()
}

/// A folha de preenchimento. Domínio próprio: não é `folha()` de ninguém, e
/// não é `no()` de nada.
pub fn vazio() -> Hash {
    let mut h = Sha256::new();
    h.update([DOM_VAZIO]);
    h.update(b"TESSERA-V1-MERKLE-VAZIO");
    h.finalize().into()
}

/// O caminho que quem vota manda junto com o voto.
#[derive(Clone, Debug, PartialEq)]
pub struct Caminho {
    /// Posição da folha na lista. Dá o lado de cada irmão.
    pub indice: u32,
    /// Irmãos da folha até a raiz, de baixo para cima.
    pub irmaos: Vec<Hash>,
}

impl Caminho {
    pub fn profundidade(&self) -> usize {
        self.irmaos.len()
    }
}

/// A árvore inteira, que só a governança monta. O contrato nunca vê isto.
#[derive(Debug)]
pub struct Arvore {
    folhas: Vec<Hash>,
    /// Níveis, do das folhas (já preenchido) até a raiz.
    niveis: Vec<Vec<Hash>>,
    n_real: usize,
}

impl Arvore {
    pub fn montar(aptos: &[Apto]) -> Result<Arvore, Erro> {
        if aptos.is_empty() {
            return Err(Erro::ListaVazia);
        }
        let folhas: Vec<Hash> = aptos.iter().map(folha).collect();
        let n_real = folhas.len();

        let mut nivel: Vec<Hash> = folhas.clone();
        let alvo = n_real.next_power_of_two();
        nivel.resize(alvo, vazio());

        let mut niveis = vec![nivel];
        while niveis.last().unwrap().len() > 1 {
            let atual = niveis.last().unwrap();
            let acima: Vec<Hash> = atual.chunks(2).map(|p| no(&p[0], &p[1])).collect();
            niveis.push(acima);
        }
        Ok(Arvore { folhas, niveis, n_real })
    }

    pub fn raiz(&self) -> Hash {
        *self.niveis.last().unwrap().first().unwrap()
    }

    /// Profundidade da árvore. `irmaos.len()` de todo caminho.
    pub fn profundidade(&self) -> usize {
        self.niveis.len() - 1
    }

    pub fn tamanho(&self) -> usize {
        self.n_real
    }

    pub fn folha_em(&self, indice: usize) -> Option<Hash> {
        self.folhas.get(indice).copied()
    }

    /// O caminho da folha `indice` até a raiz.
    pub fn caminho(&self, indice: usize) -> Result<Caminho, Erro> {
        if indice >= self.n_real {
            return Err(Erro::IndiceForaDaLista { indice, total: self.n_real });
        }
        let mut irmaos = Vec::with_capacity(self.profundidade());
        let mut i = indice;
        for nivel in &self.niveis[..self.niveis.len() - 1] {
            irmaos.push(nivel[i ^ 1]);
            i /= 2;
        }
        Ok(Caminho { indice: indice as u32, irmaos })
    }
}

/// **O que o contrato roda.**
///
/// Recebe o apto afirmado, o caminho e a raiz da proposta. Recalcula a folha a
/// partir de `endereco` e `peso` — então um peso inflado produz outra folha e
/// não chega na raiz. Essa é a defesa inteira contra peso falso.
pub fn verificar(a: &Apto, caminho: &Caminho, raiz: &Hash) -> bool {
    let mut atual = folha(a);
    let mut i = caminho.indice as usize;
    for irmao in &caminho.irmaos {
        atual = if i % 2 == 0 { no(&atual, irmao) } else { no(irmao, &atual) };
        i /= 2;
    }
    // O índice tem de ter se esgotado: um índice maior que a árvore seria
    // outro caminho para a mesma raiz.
    i == 0 && atual == *raiz
}

/// Hex de um hash, para a CLI e para os vetores de teste.
pub fn para_hex(h: &Hash) -> String {
    h.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn lista(n: usize) -> Vec<Apto> {
        (0..n)
            .map(|i| {
                let mut e = [0u8; 32];
                e[0] = (i & 0xff) as u8;
                e[1] = ((i >> 8) & 0xff) as u8;
                Apto { endereco: e, peso: 1 }
            })
            .collect()
    }

    #[test]
    fn todo_apto_prova_pertencimento() {
        for n in [1usize, 2, 3, 5, 8, 9, 100, 257] {
            let aptos = lista(n);
            let arv = Arvore::montar(&aptos).unwrap();
            let raiz = arv.raiz();
            for (i, a) in aptos.iter().enumerate() {
                let c = arv.caminho(i).unwrap();
                assert!(verificar(a, &c, &raiz), "n={} i={} falhou", n, i);
                assert_eq!(c.profundidade(), arv.profundidade());
            }
        }
    }

    #[test]
    fn quem_nao_esta_na_lista_nao_prova() {
        let aptos = lista(16);
        let arv = Arvore::montar(&aptos).unwrap();
        let raiz = arv.raiz();
        let intruso = Apto { endereco: [0xEE; 32], peso: 1 };
        for i in 0..16 {
            let c = arv.caminho(i).unwrap();
            assert!(!verificar(&intruso, &c, &raiz), "intruso passou no caminho {}", i);
        }
    }

    /// **O ataque que o módulo existe para recusar.** Quem vota afirma o peso
    /// na chamada; se o contrato aceitasse esse número, um apto de peso 1
    /// votaria com peso 1000. Como a folha é recalculada, o peso inflado
    /// produz outra folha e o caminho não fecha.
    #[test]
    fn peso_inflado_nao_chega_na_raiz() {
        let mut aptos = lista(8);
        aptos[3].peso = 1;
        let arv = Arvore::montar(&aptos).unwrap();
        let (raiz, c) = (arv.raiz(), arv.caminho(3).unwrap());

        assert!(verificar(&aptos[3], &c, &raiz));
        let mentindo = Apto { endereco: aptos[3].endereco, peso: 1000 };
        assert!(!verificar(&mentindo, &c, &raiz), "peso inflado foi aceito");
        let menos = Apto { endereco: aptos[3].endereco, peso: 0 };
        assert!(!verificar(&menos, &c, &raiz));
    }

    #[test]
    fn caminho_adulterado_e_recusado() {
        let aptos = lista(8);
        let arv = Arvore::montar(&aptos).unwrap();
        let (raiz, bom) = (arv.raiz(), arv.caminho(5).unwrap());

        // irmão trocado
        let mut c = bom.clone();
        c.irmaos[0][0] ^= 1;
        assert!(!verificar(&aptos[5], &c, &raiz));

        // índice trocado: o mesmo conjunto de irmãos, outra ordem de lados
        let mut c = bom.clone();
        c.indice = 4;
        assert!(!verificar(&aptos[5], &c, &raiz));

        // irmãos reordenados
        let mut c = bom.clone();
        c.irmaos.swap(0, 1);
        assert!(!verificar(&aptos[5], &c, &raiz));

        // caminho curto, e caminho longo
        let mut c = bom.clone();
        c.irmaos.pop();
        assert!(!verificar(&aptos[5], &c, &raiz));
        let mut c = bom.clone();
        c.irmaos.push([0u8; 32]);
        assert!(!verificar(&aptos[5], &c, &raiz));
    }

    /// O caminho de uma pessoa não serve para outra.
    #[test]
    fn caminho_nao_migra_entre_aptos() {
        let aptos = lista(8);
        let arv = Arvore::montar(&aptos).unwrap();
        let raiz = arv.raiz();
        let c = arv.caminho(2).unwrap();
        for (i, a) in aptos.iter().enumerate() {
            assert_eq!(verificar(a, &c, &raiz), i == 2);
        }
    }

    /// Separação de domínio: um nó interno não pode ser apresentado como folha,
    /// nem a folha-vazia pode ser reivindicada por alguém.
    #[test]
    fn dominios_nao_colidem() {
        let a = Apto { endereco: [7u8; 32], peso: 3 };
        let f = folha(&a);
        assert_ne!(f, no(&f, &f));
        assert_ne!(f, vazio());
        assert_ne!(vazio(), no(&vazio(), &vazio()));

        // a folha tem 37 bytes de preimagem (1+32+4) e o nó tem 65 (1+32+32):
        // nem o mesmo comprimento, nem o mesmo prefixo.
        let mut h = Sha256::new();
        h.update([DOM_FOLHA]);
        h.update([0u8; 64]);
        let como_folha: Hash = h.finalize().into();
        assert_ne!(como_folha, no(&[0u8; 32], &[0u8; 32]));
    }

    /// Preenchimento com folha-vazia, não duplicação da última. Com duplicação,
    /// a lista `[a,b,c]` e a lista `[a,b,c,c]` teriam a mesma raiz — duas
    /// listas diferentes, uma raiz só, e `c` votando duas vezes.
    #[test]
    fn preenchimento_nao_duplica_a_ultima() {
        let tres = lista(3);
        let mut quatro = lista(3);
        quatro.push(tres[2].clone());
        assert_ne!(
            Arvore::montar(&tres).unwrap().raiz(),
            Arvore::montar(&quatro).unwrap().raiz(),
            "duplicar a ultima folha daria a mesma raiz"
        );
    }

    /// A raiz depende da ordem — então a lista de aptos é um documento
    /// ordenado, e a governança publica a ordem junto com a raiz.
    #[test]
    fn raiz_muda_com_a_ordem() {
        let a = lista(4);
        let mut b = a.clone();
        b.swap(0, 1);
        assert_ne!(Arvore::montar(&a).unwrap().raiz(), Arvore::montar(&b).unwrap().raiz());
    }

    #[test]
    fn profundidade_e_logaritmica() {
        for (n, d) in [(1usize, 0usize), (2, 1), (3, 2), (4, 2), (5, 3), (256, 8), (257, 9)] {
            assert_eq!(Arvore::montar(&lista(n)).unwrap().profundidade(), d, "n={}", n);
        }
    }

    #[test]
    fn recusa_lista_vazia_e_indice_fora() {
        assert_eq!(Arvore::montar(&[]).unwrap_err(), Erro::ListaVazia);
        let arv = Arvore::montar(&lista(5)).unwrap();
        assert_eq!(
            arv.caminho(5).unwrap_err(),
            Erro::IndiceForaDaLista { indice: 5, total: 5 }
        );
        // o índice 5 existe na árvore preenchida (8 posições) mas não na lista
        assert!(arv.caminho(4).is_ok());
    }
}

/// Emissor dos vetores que a sonda 13 do contrato consome. Mesmo papel que
/// `cds::vetor`: o caminho é montado aqui, em Rust nativo, e conferido lá, em
/// Wasm. Se as duas implementações de sha256 e de ordem de bytes divergirem,
/// este é o teste que quebra (smoke B3 para o caminho de Merkle).
#[cfg(test)]
mod vetor {
    use super::*;

    /// Lista determinística de `n` aptos, a mesma dos dois lados.
    pub fn lista_fixa(n: usize) -> Vec<Apto> {
        (0..n)
            .map(|i| {
                let mut e = [0u8; 32];
                e[..8].copy_from_slice(&(i as u64).to_be_bytes());
                e[31] = 0xA7;
                Apto { endereco: e, peso: 1 }
            })
            .collect()
    }

    #[test]
    fn emitir_vetor_merkle_para_o_contrato() {
        const N: usize = 256;
        const ALVO: usize = 173;

        let aptos = lista_fixa(N);
        let arv = Arvore::montar(&aptos).unwrap();
        let c = arv.caminho(ALVO).unwrap();
        assert!(verificar(&aptos[ALVO], &c, &arv.raiz()));

        println!("\n== vetor Merkle para a sonda 13 ==");
        println!("n .............. {}", N);
        println!("profundidade ... {}", arv.profundidade());
        println!("indice ......... {}", ALVO);
        println!("endereco ....... 0x{}", para_hex(&aptos[ALVO].endereco));
        println!("peso ........... {}", aptos[ALVO].peso);
        println!("raiz ........... 0x{}", para_hex(&arv.raiz()));
        for (i, s) in c.irmaos.iter().enumerate() {
            println!("irmao[{}] ....... 0x{}", i, para_hex(s));
        }
    }
}
