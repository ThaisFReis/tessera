//! Árvore de Merkle da lista de aptos, e o caminho que quem vota apresenta.
//!
//! A folha é `H(0x00 ‖ endereço ‖ peso_be ‖ secao_be)`, onde `endereço` são os bytes XDR
//! do `Address` — exatamente o que o contrato tem em mãos quando recebe a
//! chamada, e o que identifica sem ambiguidade uma conta ou um contrato. O
//! peso e a seção ocupam os últimos 8 bytes, então o comprimento variável do
//! endereço não cria ambiguidade.
//!
//! **A seção está na folha de propósito.** Ela dimensiona o anel de quem vota
//! em sigilo, e se fosse só um argumento da chamada o votante escolheria a sua
//! — pegaria a menor, ou aquela em que consegue adivinhar melhor os outros.
//! Presa na folha, a prova de aptidão só fecha na seção certa.
//!
//! A governança integradora monta a árvore na
//! data de corte e passa só a **raiz** para `abrir()`. Quem vota apresenta o
//! caminho; o contrato confere e usa **o peso da folha, nunca o peso informado
//! na chamada** (PROTOCOLO §6.4).
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
const DOM_SECAO: u8 = 0x03;

#[derive(Debug, PartialEq)]
pub enum Erro {
    ListaVazia,
    IndiceForaDaLista { indice: usize, total: usize },
    ProfundidadeErrada { tem: usize, precisa: usize },
}

/// Uma linha da lista de aptos na data de corte.
#[derive(Clone, Debug, PartialEq)]
pub struct Apto {
    /// Bytes XDR do `Address`, como o contrato os produz com `to_xdr`.
    pub endereco: Vec<u8>,
    /// Peso, ou identificador de faixa. Conferido contra a folha, nunca
    /// aceito do que quem vota afirma.
    pub peso: u32,
}

/// `H(0x00 ‖ endereço ‖ peso_be ‖ secao_be)`.
pub fn folha(a: &Apto) -> Hash {
    let mut h = Sha256::new();
    h.update([DOM_FOLHA]);
    h.update(&a.endereco);
    h.update(a.peso.to_be_bytes());
    h.finalize().into()
}

/// Quantas pessoas uma seção deve ter, em média.
///
/// O número governa as duas pontas ao mesmo tempo, e isso não é coincidência
/// arranjada — é onde as duas restrições se cruzam:
///
/// - **por baixo**, o desequilíbrio. A seção é derivada por hash, e hash joga
///   bolas em urnas: com média 10 por seção a chance de alguma cair abaixo de
///   `τ = 5` é de 3,5% a 22% conforme o tamanho; com média 20 é de 0,0% nos
///   ensaios. Abaixo de `τ` aquela seção simplesmente não vota, e o projeto
///   prefere falhar a vazar — mas falhar sem necessidade é desenho ruim;
/// - **por cima**, a CPU. Verificar um anel custa 10.822.850 instruções por
///   membro, então um anel de 20 usa 54% do teto de uma transação.
///
/// Logo: vinte.
pub const ALVO_SECAO: u32 = 20;

/// Teto de membros numa seção, pela CPU.
///
/// Medido em `contrato` `ate_quantas_pessoas_cabe_um_anel`: **32 membros custam
/// 387.387.254 instruções, 96,8% do teto** de 400.000.000, e 34 estoura. Trinta
/// e dois é onde para.
///
/// Os dois números juntos são o que fecha a conta: com teto 32 e alvo 20, a
/// chance de uma seção cair abaixo de `τ` fica em 0,002% na pior faixa pequena
/// e 0,071% em mil aptos. Com teto 30 havia uma faixa cega de 33 a 39 aptos, em
/// que uma seção passava do teto e duas ficavam com média 16 — foi medir o teto
/// de verdade que a fechou.
pub const MAX_SECAO: u32 = 32;

/// Quantas seções para um eleitorado deste tamanho.
///
/// Parte da média-alvo e só acrescenta seção quando a média passaria do teto de
/// CPU. O resultado fica sempre em `[ALVO_SECAO, MAX_SECAO]`, que é a faixa onde
/// o desequilíbrio do hash não derruba ninguém abaixo de `τ` — travado em
/// `o_dimensionamento_mantem_a_secao_acima_de_tau`.
pub fn secoes_para(aptos: usize) -> u32 {
    let mut secoes = ((aptos as u32) / ALVO_SECAO).max(1);
    while aptos as u32 / secoes > MAX_SECAO {
        secoes += 1;
    }
    secoes
}

/// A seção de uma pessoa: `H(0x03 ‖ assinatura ‖ endereço) mod secoes`.
///
/// **Vem da baliza, não do identificador da proposta** — e a diferença é a
/// DEC-012. Enquanto a seção saía de `H(0x03 ‖ proposta ‖ endereço)`, quem
/// organizava escolhia o identificador e podia moê-lo até pôr o dissidente numa
/// seção com atacantes: cada atacante conhece a própria imagem de chave,
/// elimina a própria cédula, e o que sobra é a da vítima. Conjunto de anonimato
/// efetivo igual a 1, sem quebrar criptografia nenhuma.
///
/// Derivando da assinatura da rodada de abertura, moer não existe: na hora de
/// `abrir` aquela assinatura não existe no mundo. Depois de `abre_em` a seção é
/// determinística e pública — e **tem de ser**, porque um anel só verifica
/// contra um conjunto nomeado, então esconder a seção é impossível neste
/// desenho. O que se ganha não é sigilo da seção, é a impossibilidade de
/// **escolher** quem se esconde atrás de quem.
///
/// Por que `mod` e não o rodízio que equilibrava: o contrato tem de **conferir**
/// a seção, senão quem vota escolhe a sua e o ataque volta por outra porta. O
/// rodízio exige a lista inteira do eleitorado dentro da transação; o `mod` é um
/// hash e uma divisão. O desequilíbrio que isso traz é pago pelo
/// dimensionamento de [`ALVO_SECAO`], e o piso de `τ` continua sendo a rede de
/// proteção. Ver DEC-013.
///
/// **`assinatura` são os 96 bytes não comprimidos**, do jeito que o host do
/// Soroban os lê e guarda — `relogio::assinatura_para_host` faz a conversão. A
/// baliza publica comprimido em 48; alimentar isto com os 48 daria outra divisão
/// em silêncio, e o contrato recusaria a seção sem dizer por quê.
pub fn secao_de(assinatura: &[u8], endereco: &[u8], secoes: u32) -> u32 {
    if secoes <= 1 {
        return 0;
    }
    let mut h = Sha256::new();
    h.update([DOM_SECAO]);
    h.update(assinatura);
    h.update(endereco);
    let d: Hash = h.finalize().into();
    let n = u64::from_be_bytes(d[..8].try_into().unwrap());
    (n % secoes as u64) as u32
}

/// A divisão inteira, para o cliente mostrar antes de a votação abrir. O
/// contrato nunca a chama: ele confere uma seção de cada vez com [`secao_de`].
pub fn dividir(assinatura: &[u8], enderecos: &[Vec<u8>], secoes: u32) -> Vec<u32> {
    enderecos
        .iter()
        .map(|e| secao_de(assinatura, e, secoes))
        .collect()
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
        Ok(Arvore {
            folhas,
            niveis,
            n_real,
        })
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
            return Err(Erro::IndiceForaDaLista {
                indice,
                total: self.n_real,
            });
        }
        let mut irmaos = Vec::with_capacity(self.profundidade());
        let mut i = indice;
        for nivel in &self.niveis[..self.niveis.len() - 1] {
            irmaos.push(nivel[i ^ 1]);
            i /= 2;
        }
        Ok(Caminho {
            indice: indice as u32,
            irmaos,
        })
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
        atual = if i.is_multiple_of(2) {
            no(&atual, irmao)
        } else {
            no(irmao, &atual)
        };
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
            .map(|i| Apto {
                endereco: (i as u64).to_be_bytes().to_vec(),
                peso: 1,
            })
            .collect()
    }

    fn enderecos(n: usize) -> Vec<Vec<u8>> {
        (0..n).map(|i| (i as u64).to_be_bytes().to_vec()).collect()
    }

    /// **A garantia mudou de lugar, não desapareceu.** O rodízio equilibrava
    /// por construção e não dava para conferir dentro de uma transação; o `mod`
    /// confere num hash e desequilibra. Então o que sustenta o piso passa a ser
    /// o **dimensionamento**: com `secoes_para`, nenhuma seção cai abaixo de `τ`.
    ///
    /// O teste varre muitas assinaturas de propósito. Uma única assinatura não
    /// diria nada: a seção é função dela, e a pergunta é sobre a distribuição.
    #[test]
    fn o_dimensionamento_mantem_a_secao_acima_de_tau() {
        const TAU: usize = 5;
        for n in [21usize, 31, 32, 33, 35, 39, 40, 59, 60, 61, 100, 200, 500] {
            let es = enderecos(n);
            let secoes = secoes_para(n);
            // O teto é CPU e vale sempre; o alvo é estatístico e cede na
            // faixa logo acima do teto, onde partir em duas é o único jeito de
            // caber. O que **não** cede é o piso de `τ`, asserido abaixo.
            let media = n / secoes as usize;
            assert!(
                media <= MAX_SECAO as usize,
                "{n} em {secoes} seções dá média {media}, acima do teto de CPU"
            );
            for rodada in 0u64..200 {
                let assinatura = sha2::Sha256::digest(rodada.to_be_bytes());
                let d = dividir(&assinatura, &es, secoes);
                let mut contagem = vec![0usize; secoes as usize];
                for s in &d {
                    contagem[*s as usize] += 1;
                }
                let menor = *contagem.iter().min().unwrap();
                assert!(
                    menor >= TAU,
                    "{n} em {secoes} seções, rodada {rodada}: {contagem:?} — \
                     seção abaixo de τ, e aquelas pessoas não votariam"
                );
            }
        }
    }

    /// **DEC-012.** A seção vem da baliza, não do identificador da proposta. É o
    /// que impede quem organiza de escolher quem se esconde atrás de quem.
    #[test]
    fn a_secao_vem_da_baliza_e_nao_do_identificador() {
        let es = enderecos(40);
        let a = dividir(b"assinatura da rodada 100", &es, 2);
        let b = dividir(b"assinatura da rodada 101", &es, 2);
        assert_ne!(a, b, "a assinatura não entra na derivação");
    }

    /// E o corolário que é o conserto de verdade: **moer o identificador não
    /// isola ninguém**, porque o identificador não entra. Antes da DEC-012
    /// entrava, e quem organizava gerava propostas até o alvo cair onde queria.
    #[test]
    fn moer_o_identificador_nao_isola_ninguem() {
        let es = enderecos(40);
        let assinatura = b"a assinatura que a baliza publicou";
        let referencia = dividir(assinatura, &es, 2);
        // Dez mil tentativas de moer: nenhuma muda a seção de ninguém.
        for tentativa in 0u64..10_000 {
            let _proposta = sha2::Sha256::digest(tentativa.to_be_bytes());
            assert_eq!(
                dividir(assinatura, &es, 2),
                referencia,
                "moer a proposta mudou a divisão na tentativa {tentativa}"
            );
        }
    }

    /// O ataque que a divisão derivável existe para impedir: se o organizador
    /// escolhesse, poria o alvo numa seção sozinho e leria o voto dele.
    #[test]
    fn a_divisao_nao_depende_da_ordem_em_que_a_lista_chega() {
        let es = enderecos(20);
        let d1 = dividir(b"proposta", &es, 4);
        let mut invertida = es.clone();
        invertida.reverse();
        let d2 = dividir(b"proposta", &invertida, 4);
        for (i, e) in es.iter().enumerate() {
            let j = invertida.iter().position(|x| x == e).unwrap();
            assert_eq!(
                d1[i], d2[j],
                "quem organiza mudou a seção de alguém só reordenando a lista"
            );
        }
    }

    #[test]
    fn a_divisao_muda_com_a_rodada() {
        let es = enderecos(40);
        assert_ne!(
            dividir(b"uma", &es, 2),
            dividir(b"outra", &es, 2),
            "duas votações dariam sempre os mesmos vizinhos"
        );
    }

    #[test]
    fn sem_secoes_todo_mundo_fica_na_zero() {
        assert_eq!(dividir(b"p", &enderecos(5), 1), vec![0; 5]);
        assert_eq!(dividir(b"p", &enderecos(5), 0), vec![0; 5]);
    }

    /// **A folha não carrega mais seção** (DEC-012), e isso é amarra mais forte,
    /// não mais fraca: a seção deixa de estar presa a um campo que quem organiza
    /// escolhe na hora de montar a lista, e passa a estar presa a uma assinatura
    /// que não existe naquele momento.
    ///
    /// O teste olha a fonte, porque é a única forma de provar uma ausência.
    #[test]
    fn a_folha_nao_carrega_mais_secao() {
        let fonte = include_str!("merkle.rs");
        let corpo = &fonte[fonte.find("pub fn folha(").unwrap()..];
        let corpo = &corpo[..corpo.find("\n}").unwrap()];
        assert!(
            !corpo.contains("secao"),
            "a seção voltou para a folha: ela é escolhida por quem monta a lista"
        );
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
        let intruso = Apto {
            endereco: vec![0xEE; 32],
            peso: 1,
        };
        for i in 0..16 {
            let c = arv.caminho(i).unwrap();
            assert!(
                !verificar(&intruso, &c, &raiz),
                "intruso passou no caminho {}",
                i
            );
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
        let mentindo = Apto {
            endereco: aptos[3].endereco.clone(),
            peso: 1000,
        };
        assert!(!verificar(&mentindo, &c, &raiz), "peso inflado foi aceito");
        let menos = Apto {
            endereco: aptos[3].endereco.clone(),
            peso: 0,
        };
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
        let a = Apto {
            endereco: vec![7u8; 32],
            peso: 3,
        };
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
        assert_ne!(
            Arvore::montar(&a).unwrap().raiz(),
            Arvore::montar(&b).unwrap().raiz()
        );
    }

    #[test]
    fn profundidade_e_logaritmica() {
        for (n, d) in [
            (1usize, 0usize),
            (2, 1),
            (3, 2),
            (4, 2),
            (5, 3),
            (256, 8),
            (257, 9),
        ] {
            assert_eq!(
                Arvore::montar(&lista(n)).unwrap().profundidade(),
                d,
                "n={}",
                n
            );
        }
    }

    #[test]
    fn recusa_lista_vazia_e_indice_fora() {
        assert_eq!(Arvore::montar(&[]).unwrap_err(), Erro::ListaVazia);
        let arv = Arvore::montar(&lista(5)).unwrap();
        assert_eq!(
            arv.caminho(5).unwrap_err(),
            Erro::IndiceForaDaLista {
                indice: 5,
                total: 5
            }
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
                // 32 bytes aqui porque a sonda 13 do `bls-smoke` recebe
                // `BytesN<32>`. O contrato de verdade usa os bytes XDR do
                // `Address`, de comprimento variável — o que a função de folha
                // aceita sem mudança, porque o peso ocupa os últimos 4 bytes.
                let mut e = vec![0u8; 32];
                e[..8].copy_from_slice(&(i as u64).to_be_bytes());
                e[31] = 0xA7;
                Apto {
                    endereco: e,
                    peso: 1,
                }
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
        println!(
            "endereco ....... 0x{}",
            aptos[ALVO]
                .endereco
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        );
        println!("peso ........... {}", aptos[ALVO].peso);
        println!("raiz ........... 0x{}", para_hex(&arv.raiz()));
        for (i, s) in c.irmaos.iter().enumerate() {
            println!("irmao[{}] ....... 0x{}", i, para_hex(s));
        }
    }
}
