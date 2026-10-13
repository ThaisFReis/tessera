#!/usr/bin/env python3
"""Confere os decks (`index.html`, `index.en.html`) contra o repositório.

O `console/guarda.py` cuida da tela do console; o `docs/roteiro-confere.py`
cuida da narração do vídeo. Faltava o deck — 16 slides por idioma, texto
visível **e** notas do apresentador, que é onde moram as afirmações mais
técnicas e onde ninguém olha.

Confere quatro coisas:

1.  **Todo número grande aparece numa fonte do repositório.** "Número medido ou
    nenhum número" (§0). Se um número só existe no deck, ou ele não foi medido
    ou a medição não foi registrada — os dois são defeito.
2.  **Paridade PT/EN.** O mesmo conjunto de números nos dois idiomas; um deck
    corrigido e o outro não é o erro mais fácil de cometer aqui.
3.  **Nada que a §2 proíba**, por frase, distinguindo afirmação de negação.
4.  **Nenhuma URL prometida que responda 404** — com `--rede`, que é opt-in
    porque os portões por tarefa rodam offline.
"""

import argparse
import html
import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
DECKS = [RAIZ / "index.html", RAIZ / "index.en.html"]

FONTES = [
    "docs/SOURCES.md", "docs/SPEC.md", "docs/PROTOCOLO.md", "docs/DEMO.md",
    "README.md", "contrato/src", "core/src", "cli/src", "app/scripts",
]

NEGADORES = r"\b(?:não|nunca|nenhum\w*|nada|sem|not|never|no|none|n't)\b"
SO_NEGADAS = [
    (r"auditad[oa]|auditoria|audited|\baudit\b", "§2: nada foi auditado por terceiros"),
    (r"\bsegur[oa]\b|\bsecure\b", "§2: 'seguro' não aparece em lugar nenhum"),
    (r"pronto para produção|production[- ]ready", "§2 proíbe"),
]
PROIBIDAS = [
    (r"\btrustless\b", "§2 proíbe, inclusive para a baliza"),
    (r"\bsem confiança\b|ningu[ée]m de confian[çc]a|nobody to trust|no one to trust",
     "§2 proíbe: com fechadura, a baliza É uma suposição de confiança. "
     "Diga 'sem mesa' ou 'ninguém segurando chave'"),
    (r"sem organizador|no organiser|no organizer",
     "`abrir` recebe um endereço de governança: existe organizador"),
    (r"(?:impossível|impossible)[^.]{0,40}(?:antes da hora|before the)",
     "§2: abrir o conteúdo antes da hora é conluio de um limiar, não impossível"),
    (r"a seção[^.]{0,30}(?:é secreta|secreta)|section[^.]{0,20}is secret",
     "§2: a seção é imprevisível até a abertura, não secreta (DEC-012)"),
]

URLS = re.compile(r'href="(https://(?:thaisfreis\.github\.io|github\.com)[^"]*)"')


def fonte_completa():
    buf = []
    for f in FONTES:
        alvo = RAIZ / f
        if alvo.is_dir():
            for p in alvo.rglob("*"):
                if p.suffix in {".rs", ".mjs", ".ts", ".md"}:
                    buf.append(p.read_text(encoding="utf-8", errors="ignore"))
        elif alvo.exists():
            buf.append(alvo.read_text(encoding="utf-8", errors="ignore"))
    return re.sub(r"[.,_\s]", "", " ".join(buf))


def partes(caminho):
    h = caminho.read_text(encoding="utf-8")
    corpo = re.sub(r"<style.*?</style>|<script.*?</script>", "", h, flags=re.S)
    visivel = html.unescape(re.sub(r"<[^>]+>", " ", corpo))
    notas = " ".join(
        html.unescape(m) for m in re.findall(r'data-notes="([^"]*)"', h)
    )
    return h, visivel, notas


def numeros(texto):
    """Números com separador de milhar — os que afirmam medição."""
    return {re.sub(r"[.,]", "", n) for n in
            re.findall(r"\b\d{1,3}(?:[.,]\d{3})+\b", texto)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rede", action="store_true",
                    help="confere também se as URLs prometidas respondem")
    args = ap.parse_args()

    fontes = fonte_completa()
    falhas = []
    por_deck = {}

    for caminho in DECKS:
        if not caminho.exists():
            falhas.append(f"{caminho.name}: não existe")
            continue
        bruto, visivel, notas = partes(caminho)
        todo = visivel + " " + notas
        nome = caminho.name

        nums = numeros(todo)
        por_deck[nome] = nums
        for n in sorted(nums):
            if n not in fontes:
                falhas.append(
                    f"{nome}: {n} não aparece em nenhuma fonte do repositório "
                    "— medição não registrada, ou número inventado"
                )

        for padrao, porque in PROIBIDAS:
            for m in re.finditer(padrao, todo, re.I):
                trecho = " ".join(todo[m.start():m.end() + 60].split())
                falhas.append(f"{nome}: {trecho!r} — {porque}")
        for padrao, porque in SO_NEGADAS:
            for frase in re.split(r"(?<=[.?!])\s+", todo):
                if re.search(padrao, frase, re.I) and not re.search(
                    NEGADORES, frase, re.I
                ):
                    falhas.append(f"{nome}: {frase.strip()[:90]!r} — {porque}")

        print(f"  {nome:<16} {len(nums):3d} números medidos")

    nomes = list(por_deck)
    if len(nomes) == 2:
        so_a = por_deck[nomes[0]] - por_deck[nomes[1]]
        so_b = por_deck[nomes[1]] - por_deck[nomes[0]]
        for n in sorted(so_a):
            falhas.append(f"paridade: {n} está em {nomes[0]} e não em {nomes[1]}")
        for n in sorted(so_b):
            falhas.append(f"paridade: {n} está em {nomes[1]} e não em {nomes[0]}")

    if args.rede:
        import urllib.request
        import urllib.error
        vistas = set()
        for caminho in DECKS:
            if not caminho.exists():
                continue
            for url in URLS.findall(caminho.read_text(encoding="utf-8")):
                if url in vistas:
                    continue
                vistas.add(url)
                try:
                    req = urllib.request.Request(url, method="HEAD")
                    codigo = urllib.request.urlopen(req, timeout=15).status
                except urllib.error.HTTPError as e:
                    codigo = e.code
                except Exception as e:
                    codigo = f"erro: {e}"
                marca = "✓" if codigo == 200 else "✗"
                print(f"  {marca} {codigo}  {url}")
                if codigo != 200:
                    falhas.append(
                        f"o deck promete {url}, que responde {codigo}"
                    )

    print()
    if falhas:
        for f in falhas:
            print(f"  ✗ {f}")
        print(f"\n  {len(falhas)} problema(s)")
        return 1
    print("  ✓ todo número do deck tem fonte, os dois idiomas batem, "
          "e nada que a §2 proíba")
    return 0


if __name__ == "__main__":
    sys.exit(main())
