//! Tessera — cliente de linha de comando.
//!
//! Sete comandos, e os cinco que a demo não pode perder são `cedula`, `votar`,
//! `queimar`, `apurar --forcar-total` e `verificar`.

mod cadeia;
mod cedula;
mod chave;
mod comandos;
mod estado;
mod recibo;
mod tela;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tessera", about = "Voto secreto para qualquer governança, na Stellar.")]
struct Cli {
    #[command(subcommand)]
    comando: Comando,
}

#[derive(Subcommand)]
enum Comando {
    /// Abre uma votação. Quem chama é a governança.
    Abrir {
        #[arg(long)]
        proposta: String,
        /// Uma por pergunta: `"texto | opção, opção | sigilosa"`.
        ///
        /// A natureza é `sigilosa` ou `publica`, e cai em `sigilosa` se você
        /// omitir. Uma cédula toda sigilosa é a confidencial; misturar as duas
        /// é a semiconfidencial.
        ///
        /// Com `--opcoes`, `--pergunta` volta a ser só o texto da única
        /// pergunta, sigilosa — é a forma curta de sempre.
        #[arg(long)]
        pergunta: Vec<String>,
        /// Forma curta para a cédula de uma pergunta só, sigilosa.
        /// Lista separada por vírgula, ou arquivo com uma por linha.
        #[arg(long)]
        opcoes: Option<String>,
        /// Identidades da `stellar keys`, por vírgula ou em arquivo.
        #[arg(long)]
        aptos: String,
        #[arg(long)]
        mesa: String,
        /// Quantas assinaturas da mesa apuram.
        #[arg(short = 'k', long, default_value = "3")]
        limiar: u32,
        /// `2h`, `30m` ou um número de ledgers.
        #[arg(long, default_value = "2h")]
        prazo: String,
        #[arg(long, env = "TESSERA_CONTRATO")]
        contrato: String,
        #[arg(long, default_value = "testnet")]
        rede: String,
        #[arg(long, default_value = "urna-smoke")]
        governanca: String,
    },
    /// Mostra o que a rede guardaria. **Não vota.**
    Cedula {
        #[arg(long)]
        proposta: String,
        #[arg(long)]
        identidade: String,
    },
    /// Registra o voto.
    Votar {
        #[arg(long)]
        proposta: String,
        /// Uma por pergunta, na ordem da cédula. Numa cédula de uma pergunta
        /// só, basta uma.
        #[arg(long)]
        opcao: Vec<String>,
        #[arg(long)]
        identidade: String,
        /// Abre a cédula **inteira** em claro — inclusive as perguntas
        /// sigilosas. É a revelação voluntária.
        #[arg(long)]
        publico: bool,
    },
    /// Sobrescreve o recibo. Depois disso nem você prova em que votou.
    Queimar {
        #[arg(long)]
        identidade: String,
    },
    /// Dois números grandes, para a sala.
    Status {
        #[arg(long)]
        proposta: String,
    },
    /// A mesa reconstrói, afirma, e o contrato confere.
    Apurar {
        #[arg(long)]
        proposta: String,
        /// Faz a mesa mentir, para gravar a recusa.
        #[arg(long)]
        forcar_total: Option<String>,
    },
    /// Relê e refaz tudo. Não confia em ninguém.
    Verificar {
        #[arg(long)]
        proposta: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let r = match &cli.comando {
        Comando::Abrir { proposta, pergunta, opcoes, aptos, mesa, limiar, prazo, contrato, rede, governanca } =>
            comandos::abrir(proposta, pergunta, opcoes.as_deref(), aptos, mesa, *limiar, prazo, contrato, rede, governanca),
        Comando::Cedula { proposta, identidade } => comandos::mostrar_cedula(proposta, identidade),
        Comando::Votar { proposta, opcao, identidade, publico } => comandos::votar(proposta, opcao, identidade, *publico),
        Comando::Queimar { identidade } => comandos::queimar(identidade),
        Comando::Status { proposta } => comandos::status(proposta),
        Comando::Apurar { proposta, forcar_total } => comandos::apurar(proposta, forcar_total.as_deref()),
        Comando::Verificar { proposta } => comandos::verificar(proposta),
    };
    if let Err(e) = r {
        tela::recusa(&e);
        println!();
        std::process::exit(1);
    }
}
