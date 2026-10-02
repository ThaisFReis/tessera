#!/usr/bin/env python3
"""A guarda dos recortes de código da demo.

Os terminais de `console/index.html` mostram trechos de `core/src/*.rs` e
companhia. São **cópias**, e cópia envelhece: se o crate mudar e o recorte não
mudar junto, a demo passa a mentir — com cara de código-fonte, que é o pior
jeito de mentir.

Esta guarda confere duas coisas:

  1. todo arquivo citado no cabeçalho de um terminal está declarado em
     `ANCORAS`, e existe no repositório;
  2. todo trecho listado em `ANCORAS` ainda aparece, literalmente, no arquivo.

Constantes entram com o valor inteiro de propósito. Trocar τ de 5 para 3 sem
mexer na tela é exatamente o erro que isto existe para pegar.

    python3 console/guarda.py

Sai com 0 se a tela ainda corresponde ao código, e 1 se alguma âncora sumiu.
"""

import json
import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
PAGINA = RAIZ / "console" / "index.html"


def ler_ancoras(fonte):
    """Extrai o literal `const ANCORAS = {...};` e o lê como JSON.

    O bloco é escrito em JS, mas no subconjunto que também é JSON — strings com
    aspas duplas e nada de vírgula sobrando. A única diferença é a vírgula final
    de cada lista, que o JS aceita; ela é removida aqui.
    """
    m = re.search(r"const ANCORAS = (\{.*?\n\});", fonte, re.S)
    if not m:
        sys.exit("não achei o bloco `const ANCORAS` em console/index.html")
    # O bloco é JS: aceita comentário de linha e vírgula sobrando, e os dois
    # precisam sair antes de virar JSON. Comentário dentro de string não
    # acontece aqui — as âncoras são trechos de código, não URLs.
    bruto = re.sub(r"^\s*//.*$", "", m.group(1), flags=re.M)
    bruto = re.sub(r",(\s*[\]\}])", r"\1", bruto)
    try:
        return json.loads(bruto)
    except json.JSONDecodeError as e:
        sys.exit(f"o bloco ANCORAS não é JSON válido: {e}")


def arquivos_citados(fonte):
    """Os arquivos que aparecem no cabeçalho de cada terminal.

    O cabeçalho pode citar mais de um, separado por ` · `, e o segundo costuma
    vir só com o nome — `core/src/merkle.rs · shamir.rs`. A pasta do primeiro
    completa os seguintes.
    """
    citados = set()
    for cab in re.findall(r'terminalMecanica\("([^"]*)"', fonte):
        # Um painel sem arquivo (o vazio que espera a proposta) não declara nada.
        if "." not in cab:
            continue
        partes = [x.strip() for x in cab.split("·")]
        pasta = str(Path(partes[0]).parent)
        for parte in partes:
            citados.add(parte if "/" in parte else f"{pasta}/{parte}")
    return citados


def main():
    fonte = PAGINA.read_text(encoding="utf-8")
    ancoras = ler_ancoras(fonte)
    citados = arquivos_citados(fonte)

    faltas = []

    # 1. todo arquivo citado está declarado
    for arq in sorted(citados - set(ancoras)):
        faltas.append(f"{arq} aparece num terminal mas não está em ANCORAS")

    # 2. toda âncora ainda existe no arquivo
    conferidas = 0
    for arq, trechos in ancoras.items():
        caminho = RAIZ / arq
        if not caminho.exists():
            faltas.append(f"{arq} não existe")
            continue
        texto = caminho.read_text(encoding="utf-8")
        for t in trechos:
            conferidas += 1
            if t not in texto:
                faltas.append(f"{arq} não contém mais «{t}»")

    print()
    print("  TESSERA · guarda dos recortes")
    print("  " + "─" * 68)
    print()
    print(f"  Arquivos .... {len(ancoras)}")
    print(f"  Âncoras ..... {conferidas}")
    print()

    if faltas:
        for f in faltas:
            print(f"  ✗ {f}")
        print()
        print("  A tela afirma coisas que o código não diz mais.")
        print("  Conserte o recorte em console/index.html, ou a âncora.")
        print()
        return 1

    print("  ✓ os recortes da tela ainda batem com o código")
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
