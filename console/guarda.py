#!/usr/bin/env python3
"""Guarda dos acoplamentos da tela.

A versão anterior conferia **recortes**: a coluna da direita copiava trechos de
`core/src/*.rs`, e a guarda avisava quando a cópia envelhecia. Os recortes
saíram da tela — não há mais cópia a envelhecer, e conferir que um símbolo
existe num arquivo deixou de provar coisa alguma sobre o que o jurado lê.

O que sobrou é mais estreito e mais verdadeiro: a tela ainda **afirma** números
e garantias que vêm do código, e nada liga os dois lados além de mim ter
digitado o mesmo valor duas vezes. `16` no `if` do formulário é `MAX_OPCOES`.
`4 de 7` no painel 9 só é uma recusa porque `TAU` vale 5. `320 B` é o tamanho
da disjuntiva. Qualquer um desses pode mudar no Rust sem que nada quebre — e a
demo passa a mentir com cara de medição.

Cada acoplamento é conferido **dos dois lados**: a afirmação tem de continuar na
tela, e o fato tem de continuar no código. Se um sumir sem o outro, é falha, e a
mensagem diz qual afirmação ficou órfã.

Três garantias são de **ausência** — o que a demo promete é que algo não existe.
Essas não têm como ser conferidas por amostragem: ou o símbolo sumiu, ou a
promessa é falsa.

    python3 console/guarda.py
"""

import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
TELA = "console/index.html"
PONTE = "console/ponte.py"
CLI = "cli/src/main.rs"

# Cada entrada: o que a tela afirma, o literal que afirma, e o fato no código.
ACOPLAMENTOS = [
    {
        "afirma": "o formulário recusa acima de 16 opções sigilosas",
        "tela": ["total>16", "O máximo é 16 opções sigilosas no total."],
        "fonte": [("contrato/src/tipos.rs", "pub const MAX_OPCOES: u32 = 16;")],
    },
    {
        "afirma": "4 de 7 fica abaixo do quórum e a apuração é recusada",
        # Se TAU cair para 3, o painel 9 roda, apura, e a cena inteira — a que
        # explica por que o contrato prefere não publicar — vira um sucesso
        # silencioso. Nenhum teste pega isso: a tela é que está errada.
        "tela": ["ponte.identidades.slice(0, 4)", "Rodar com 4 de 7 e tentar apurar →"],
        "fonte": [("contrato/src/tipos.rs", "pub const TAU: u32 = 5;")],
    },
    {
        "afirma": "a recusa por quórum é Error(Contract, #19)",
        "tela": ["Error(Contract, #19)"],
        "fonte": [("contrato/src/tipos.rs", "AnonimatoInsuficiente = 19,")],
    },
    {
        "afirma": "cada disjuntiva ocupa 320 B",
        "tela": ["320 B"],
        "fonte": [("core/src/cds.rs", "pub const TAMANHO: usize = 320;")],
    },
    {
        "afirma": "cada prova de soma ocupa 128 B, e cada compromisso 96 B",
        "tela": ["128 B", "96 B cada", "96 bytes"],
        "fonte": [("core/src/soma.rs", "pub const TAMANHO: usize = 96 + 32;")],
    },
    {
        "afirma": "não existe voto aberto, então não há partição a induzir",
        "tela": ["Sem voto aberto não há partição a induzir"],
        "fonte": [("contrato/src/lib.rs", "`votar_publico` existia aqui e foi **removido**")],
        "ausente": [("contrato/src/lib.rs", "pub fn votar_publico")],
    },
    {
        "afirma": "votar não grava nada em disco",
        "tela": ["`votar` não chama mais recibo::gravar"],
        "fonte": [("cli/src/recibo.rs", "nenhum_comando_grava_recibo")],
        "ausente": [("cli/src/comandos.rs", "recibo::gravar")],
    },
    {
        "afirma": "esta página não roda BLS12-381",
        # A única afirmação da demo sobre ela mesma, e a mais fácil de tornar
        # falsa sem perceber: basta alguém importar uma biblioteca para
        # "conferir no cliente".
        "tela": ["esta página não roda BLS12-381"],
        "fonte": [],
        "ausente": [(TELA, "bls12_381"), (TELA, "@noble"), (TELA, "pairing")],
    },
]


def tem(arquivo, trecho):
    caminho = RAIZ / arquivo
    if not caminho.exists():
        return None
    return trecho in caminho.read_text(encoding="utf-8")


def flags_da_ponte():
    """Toda `--flag` que a ponte manda tem de existir na CLI.

    É a falha que já aconteceu em cima da hora: a ponte mandou `--abre_em` para
    um binário que não conhecia o argumento, e a tela só mostrou `unexpected
    argument` no meio de uma demonstração.
    """
    ponte = (RAIZ / PONTE).read_text(encoding="utf-8")
    cli = (RAIZ / CLI).read_text(encoding="utf-8")
    faltas = []
    vistas = sorted(set(re.findall(r'"--([a-z][a-z0-9-]*)"', ponte)))
    for flag in vistas:
        campo = flag.replace("-", "_")
        if not re.search(rf"^\s*{re.escape(campo)}:", cli, re.M):
            faltas.append(f"a ponte manda --{flag} e {CLI} não tem o campo `{campo}`")
    return vistas, faltas


def main():
    faltas = []
    conferidos = 0

    for a in ACOPLAMENTOS:
        quebrou = []
        for t in a["tela"]:
            conferidos += 1
            if not tem(TELA, t):
                quebrou.append(f"a tela não diz mais «{t}»")
        for arq, t in a.get("fonte", []):
            conferidos += 1
            achou = tem(arq, t)
            if achou is None:
                quebrou.append(f"{arq} não existe")
            elif not achou:
                quebrou.append(f"{arq} não contém mais «{t}»")
        for arq, t in a.get("ausente", []):
            conferidos += 1
            achou = tem(arq, t)
            if achou is None:
                quebrou.append(f"{arq} não existe")
            elif achou:
                quebrou.append(f"{arq} voltou a conter «{t}»")
        if quebrou:
            faltas.append((a["afirma"], quebrou))

    vistas, faltas_flags = flags_da_ponte()
    conferidos += len(vistas)
    if faltas_flags:
        faltas.append(("a ponte e a CLI falam a mesma língua", faltas_flags))

    print()
    print("  TESSERA · guarda dos acoplamentos")
    print("  " + "─" * 68)
    print()
    print(f"  Afirmações ... {len(ACOPLAMENTOS)}")
    print(f"  Conferências . {conferidos}")
    print(f"  Flags ........ {len(vistas)} da ponte, todas na CLI"
          if not faltas_flags else f"  Flags ........ {len(vistas)} da ponte")
    print()

    if faltas:
        for afirma, motivos in faltas:
            print(f"  ✗ {afirma}")
            for m in motivos:
                print(f"      {m}")
        print()
        print("  Um lado mudou sem o outro. A tela afirma o que o código não")
        print("  sustenta mais — conserte a afirmação, ou a âncora.")
        print()
        return 1

    print("  ✓ tudo o que a tela afirma, o código ainda sustenta")
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
