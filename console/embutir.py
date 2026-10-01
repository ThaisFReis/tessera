#!/usr/bin/env python3
"""Embute uma rodada real no console, para a página abrir mostrando algo.

    python3 console/embutir.py demo/estado/estatuto.json

O estado nunca contém `r` — essa é a razão de ele ter sido desenhado assim
(UX-CLI §9) — mas o script confere antes de embutir, porque este é o único
lugar do projeto em que um arquivo local vira página pública.
"""
import json
import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent
PROIBIDO = ('"r"', "acaso", "aleatori", "segredo", '"chave"')


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    origem = Path(sys.argv[1])
    dados = json.loads(origem.read_text(encoding="utf-8"))

    bruto = json.dumps(dados, ensure_ascii=False)
    for p in PROIBIDO:
        if p in bruto:
            print(f"ERRO: o estado contém {p!r}. Nada de segredo vai para a página.")
            return 1

    compacto = json.dumps(dados, separators=(",", ":"), ensure_ascii=False)
    alvo = RAIZ / "index.html"
    html = alvo.read_text(encoding="utf-8")

    # Substitui SÓ o conteúdo do bloco de dados. Trocar por nome de marcador já
    # mordeu uma vez: o mesmo marcador existia dentro do JavaScript e as duas
    # ocorrências foram trocadas, quebrando a página inteira.
    padrao = re.compile(
        r'(<script id="instantaneo" type="application/json">)(.*?)(</script>)',
        re.S,
    )
    novo, n = padrao.subn(lambda m: m.group(1) + compacto + m.group(3), html, count=1)
    if n != 1:
        print("ERRO: não achei o bloco de dados no index.html")
        return 1

    alvo.write_text(novo, encoding="utf-8")
    (RAIZ / "exemplo").mkdir(exist_ok=True)
    (RAIZ / "exemplo" / f"{dados['proposta']}.json").write_text(
        json.dumps(dados, indent=1, ensure_ascii=False), encoding="utf-8"
    )
    print(f"embutido: {dados['proposta']} · {len(dados['votos'])} votos · {len(novo)} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
