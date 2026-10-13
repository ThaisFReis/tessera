#!/usr/bin/env python3
"""Confere os roteiros de vídeo contra o que o repositório sustenta.

Não substitui `console/guarda.py`, que cuida da landing. Este cuida de
`docs/VIDEO-PITCH.md` e `.en.md`, onde a narração é escrita à mão e por isso
pode divergir do contrato sem ninguém perceber.

Três classes de erro, todas cometidas de verdade neste projeto:

1.  Etiqueta do ElevenLabs partida por quebra de linha — `[long\\npause]` não é
    reconhecida e sai falada.
2.  Citação do roteiro diferente do bloco que vai ao sintetizador — o ator lê
    uma coisa, a voz gera outra.
3.  Palavra proibida pela §2 do SPEC, ou confusão entre "prazo" e "fechadura
    de tempo": toda votação tem prazo, só algumas têm fechadura.
"""

import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
ROTEIROS = [RAIZ / "docs/VIDEO-PITCH.md", RAIZ / "docs/VIDEO-PITCH.en.md"]

# §2 do SPEC: não aparecem em lugar nenhum.
PROIBIDAS = [
    (r"\btrustless\b", "§2 proíbe chamar qualquer coisa de trustless"),
    (r"pronto para produção|production[- ]ready", "§2 proíbe"),
    (r"\bsem confiança\b", "§2 proíbe — diga 'sem mesa' ou 'ninguém tem chave'"),
    (r"sem organizador|no organiser|no organizer",
     "`abrir` recebe um endereço de governança: existe organizador. O que não "
     "existe é mesa"),
    (r"(?:a votação|the vote)[^.]{0,30}(?:não tem prazo|has no deadline)",
     "toda votação tem prazo (INV-18); o que varia é ter fechadura de tempo"),
]

# Palavras que a §2 só proíbe quando **afirmadas**. Dizer "não foi auditado" é
# obrigatório; dizer "foi auditado" é mentira. A diferença é um negador na mesma
# frase, e lookbehind não alcança ("none of this has been audited").
NEGADORES = r"\b(?:não|nunca|nenhum\w*|nada|sem|not|never|no|none|n't)\b"
SO_NEGADAS = [
    (r"auditad[oa]|auditoria|audited|audit\b",
     "§2: nada aqui foi auditado por terceiros"),
    (r"\bsegur[oa]\b|\bsecure\b",
     "§2: 'seguro' não aparece em lugar nenhum"),
]

# Afirmações que só podem aparecer junto da sua condição.
PAREADAS = [
    (r"\bninguém abre\b|\bnobody opens\b",
     r"mesa|board",
     "dizer que ninguém abre exige dizer que sem mesa e sem fechadura não sai "
     "resultado nenhum — ver `assembleia_sem_mesa_abre_e_nao_apura`"),
]


def blocos_tts(texto):
    return dict(
        (n, corpo)
        for n, corpo in re.findall(
            r"### (\d+) · [^\n]+\n\n```\n(.*?)\n```", texto, re.S
        )
    )


def citacao(texto, n):
    secoes = [s for s in re.split(r"\n---\n", texto) if re.match(r"\n?## %s\. " % n, s)]
    if not secoes:
        return None
    linhas, dentro = [], False
    for linha in secoes[0].split("\n"):
        if linha.startswith("> **"):
            break
        if linha.startswith(">"):
            dentro = True
            linhas.append(linha[1:].strip())
        elif dentro:
            break
    return " ".join(" ".join(linhas).split())


def sem_etiquetas(s):
    return " ".join(re.sub(r"\[[a-z ]+\]", "", s).split())


def confere(caminho):
    texto = caminho.read_text(encoding="utf-8")
    nome = caminho.name
    falhas = []

    partidas = re.findall(r"\[[a-z]+\n[a-z]+\]", texto)
    for p in partidas:
        falhas.append(f"{nome}: etiqueta partida por quebra de linha: {p!r}")

    tts = blocos_tts(texto)
    if len(tts) != 8:
        falhas.append(f"{nome}: {len(tts)} blocos de narração, esperava 8")
    for n, corpo in tts.items():
        cit = citacao(texto, n)
        if cit is None:
            falhas.append(f"{nome} §{n}: citação do roteiro não encontrada")
        elif cit != sem_etiquetas(corpo):
            falhas.append(f"{nome} §{n}: roteiro e narração divergem")

    fala = " ".join(sem_etiquetas(c) for c in tts.values())
    for padrao, porque in PROIBIDAS:
        if re.search(padrao, fala, re.I):
            falhas.append(f"{nome}: fala contém {padrao!r} — {porque}")
    for padrao, porque in SO_NEGADAS:
        for frase in re.split(r"(?<=[.?!])\s+", fala):
            if re.search(padrao, frase, re.I) and not re.search(
                NEGADORES, frase, re.I
            ):
                falhas.append(f"{nome}: {frase.strip()!r} afirma — {porque}")
    for gatilho, exigido, porque in PAREADAS:
        if re.search(gatilho, fala, re.I) and not re.search(exigido, fala, re.I):
            falhas.append(f"{nome}: {porque}")

    palavras = len(fala.split())
    return falhas, palavras


def main():
    todas = []
    for caminho in ROTEIROS:
        if not caminho.exists():
            todas.append(f"{caminho.name}: não existe")
            continue
        falhas, palavras = confere(caminho)
        todas += falhas
        s = round(palavras / 160 * 60)
        print(f"  {caminho.name:<22} {palavras:4d} palavras · {s // 60}min{s % 60:02d}")

    print()
    if todas:
        for f in todas:
            print(f"  ✗ {f}")
        print(f"\n  {len(todas)} problema(s)")
        return 1
    print("  ✓ roteiro e narração batem, e nada que a §2 proíba")
    return 0


if __name__ == "__main__":
    sys.exit(main())
