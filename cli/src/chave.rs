//! Strkey da Stellar, e o XDR de `Address` que o contrato lê.
//!
//! O contrato monta a folha de Merkle com `H(0x00 ‖ addr_xdr ‖ peso)` e o
//! contexto das provas com `proposta ‖ addr_xdr ‖ opção`. Esses bytes **têm**
//! de ser os mesmos aqui e lá, então o formato está travado em teste contra o
//! que `soroban_sdk::Address::to_xdr` produz de verdade.

#[derive(Debug, PartialEq)]
pub enum Erro {
    TamanhoErrado(usize),
    CaractereInvalido(char),
    VersaoDesconhecida(u8),
    ChecksumNaoConfere,
}

const ALFABETO: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const VER_CONTA: u8 = 6 << 3; // 'G'
const VER_CONTRATO: u8 = 2 << 3; // 'C'

fn crc16(dados: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in dados {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

fn base32_decodificar(s: &str) -> Result<Vec<u8>, Erro> {
    let mut saida = Vec::with_capacity(s.len() * 5 / 8);
    let (mut buf, mut bits) = (0u32, 0u32);
    for c in s.chars() {
        if c == '=' {
            break;
        }
        let v = ALFABETO
            .iter()
            .position(|&a| a as char == c)
            .ok_or(Erro::CaractereInvalido(c))? as u32;
        buf = (buf << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            saida.push((buf >> bits) as u8);
        }
    }
    Ok(saida)
}

/// `G…`/`C…` → `(versão, 32 bytes)`.
pub fn decodificar(s: &str) -> Result<(u8, [u8; 32]), Erro> {
    let bruto = base32_decodificar(s.trim())?;
    if bruto.len() != 35 {
        return Err(Erro::TamanhoErrado(bruto.len()));
    }
    let versao = bruto[0];
    if versao != VER_CONTA && versao != VER_CONTRATO {
        return Err(Erro::VersaoDesconhecida(versao));
    }
    let esperado = u16::from_le_bytes([bruto[33], bruto[34]]);
    if crc16(&bruto[..33]) != esperado {
        return Err(Erro::ChecksumNaoConfere);
    }
    let mut chave = [0u8; 32];
    chave.copy_from_slice(&bruto[1..33]);
    Ok((versao, chave))
}

/// **Os 44 bytes que `Address::to_xdr` produz.**
///
/// É o XDR de um `ScVal::Address`, e não o da conta sozinha:
///
/// ```text
/// 00000012   SCV_ADDRESS (18)
/// 0000000x   0 = conta, 1 = contrato
/// 00000000   PUBLIC_KEY_TYPE_ED25519   (só para conta)
/// <32 bytes> a chave
/// ```
///
/// Medido contra o SDK, não deduzido da especificação XDR.
pub fn xdr_de_endereco(s: &str) -> Result<Vec<u8>, Erro> {
    let (versao, chave) = decodificar(s)?;
    let mut v = Vec::with_capacity(44);
    v.extend_from_slice(&18u32.to_be_bytes()); // SCV_ADDRESS
    if versao == VER_CONTA {
        v.extend_from_slice(&0u32.to_be_bytes()); // SC_ADDRESS_TYPE_ACCOUNT
        v.extend_from_slice(&0u32.to_be_bytes()); // PUBLIC_KEY_TYPE_ED25519
    } else {
        v.extend_from_slice(&1u32.to_be_bytes()); // SC_ADDRESS_TYPE_CONTRACT
    }
    v.extend_from_slice(&chave);
    Ok(v)
}

#[cfg(test)]
mod testes {
    use super::*;

    const CONTA: &str = "GCZC4HW5TA2FSFPSFX3SJTOCDFQ6KZBHDHEG5O32STRX4PQXUX4NLGWJ";

    /// **O vetor que importa.** Medido com `Address::to_xdr` no SDK, não
    /// deduzido: se este teste quebrar, a folha de Merkle e o contexto das
    /// provas divergem do contrato, e nenhum voto fecha.
    #[test]
    fn xdr_bate_com_o_que_o_sdk_produz() {
        const ESPERADO: &str = "000000120000000000000000\
b22e1edd98345915f22df724cdc21961e5642719c86ebb7a94e37e3e17a5f8d5";
        let x = xdr_de_endereco(CONTA).unwrap();
        assert_eq!(x.len(), 44);
        let hex: String = x.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, ESPERADO, "o XDR do endereco DIVERGE do SDK");
    }

    #[test]
    fn recusa_strkey_corrompida() {
        let mut ruim: Vec<char> = CONTA.chars().collect();
        ruim[10] = if ruim[10] == 'A' { 'B' } else { 'A' };
        let s: String = ruim.into_iter().collect();
        assert_eq!(decodificar(&s), Err(Erro::ChecksumNaoConfere));
        assert!(decodificar("GCZC").is_err());
        assert!(decodificar("1CZC4HW5TA2FSFPSFX3SJTOCDFQ6KZBHDHEG5O32STRX4PQXUX4NLGWJ").is_err());
    }

    #[test]
    fn contrato_tem_discriminante_proprio() {
        const CONTRATO: &str = "CBD5QTEJPKQGNLFBEGVRXDKJ6CUBGYS2CHEUFEXPERR7W7TXBFH4X43W";
        let x = xdr_de_endereco(CONTRATO).unwrap();
        assert_eq!(x.len(), 40, "contrato nao tem o campo de tipo de chave");
        assert_eq!(&x[4..8], &1u32.to_be_bytes());
    }
}
