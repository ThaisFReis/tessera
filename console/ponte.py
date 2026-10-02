#!/usr/bin/env python3
"""A ponte entre a tela e a rede.

A demo é uma página estática, e página estática não assina transação. A ponte
resolve isso do jeito mais curto que existe: ela chama a CLI, que é quem já sabe
provar, assinar e enviar.

    navegador  →  ponte (localhost)  →  tessera  →  stellar  →  testnet

Nada é simulado. Cada clique na tela vira uma transação de verdade na testnet,
com hash conferível. O que a tela mostra depois é o `estado/<proposta>.json` que
a própria CLI gravou.

    python3 console/ponte.py          # e abra http://127.0.0.1:8777

Escuta **só** em 127.0.0.1. As chaves ficam onde sempre estiveram, na
`stellar keys` da máquina — a ponte não as vê, não as pede e não as transporta.
"""

import json
import os
import re
import subprocess
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
DEMO = RAIZ / "demo"
CONSOLE = RAIZ / "console"
BIN = RAIZ / "cli" / "target" / "release" / "tessera"

PORTA = int(os.environ.get("TESSERA_PORTA", "8780"))
CONTRATO = os.environ.get("TESSERA_CONTRATO", "")

NOME = re.compile(r"^[a-zA-Z0-9_-]{1,64}$")

TIPOS = {
    ".html": "text/html; charset=utf-8",
    ".js": "application/javascript; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".json": "application/json; charset=utf-8",
    ".svg": "image/svg+xml",
    ".png": "image/png",
}


class Recusa(Exception):
    """Entrada que a ponte não aceita. Nunca chega à CLI."""


def nome(v, campo):
    if not isinstance(v, str) or not NOME.match(v):
        raise Recusa(f"{campo} inválido")
    return v


def rodar(args):
    """Chama a CLI de dentro de `demo/`, que é onde ela guarda o estado.

    Sem shell: os argumentos vão como lista, então nada que venha do navegador
    é interpretado como comando.
    """
    if not BIN.exists():
        raise Recusa(
            "A CLI não está compilada. Rode:  cd cli && cargo build --release"
        )
    amb = dict(os.environ)
    if CONTRATO:
        amb["TESSERA_CONTRATO"] = CONTRATO
    p = subprocess.run(
        [str(BIN)] + args,
        cwd=str(DEMO),
        capture_output=True,
        text=True,
        timeout=180,
        env=amb,
    )
    return p.returncode, (p.stdout or "") + (p.stderr or "")


def estado(proposta):
    f = DEMO / "estado" / f"{proposta}.json"
    if not f.exists():
        return None
    try:
        return json.loads(f.read_text(encoding="utf-8"))
    except Exception:
        return None


# ---------- os quatro verbos ----------


def abrir(c):
    proposta = nome(c.get("proposta"), "proposta")
    perguntas = c.get("perguntas")
    if not isinstance(perguntas, list) or not 1 <= len(perguntas) <= 8:
        raise Recusa("de 1 a 8 perguntas")

    args = ["abrir", "--proposta", proposta]
    sigilosas = 0
    for i, p in enumerate(perguntas, 1):
        texto = (p.get("texto") or "").strip()
        opcoes = [o.strip() for o in (p.get("opcoes") or []) if o.strip()]
        if "|" in texto:
            raise Recusa(f"a pergunta {i} não pode conter '|'")
        if not texto or len(opcoes) < 2 or len(opcoes) > 16:
            raise Recusa(f"a pergunta {i} precisa de texto e de 2 a 16 opções")
        if any("|" in o or "," in o for o in opcoes):
            raise Recusa(f"as opções da pergunta {i} não podem conter '|' nem ','")
        confidencial = bool(p.get("confidencial"))
        if confidencial:
            sigilosas += len(opcoes)
        natureza = "sigilosa" if confidencial else "publica"
        args += ["--pergunta", f"{texto} | {', '.join(opcoes)} | {natureza}"]

    # O contrato recusaria isso de todo jeito; recusar aqui dá uma mensagem
    # legível em vez de um código de erro.
    if sigilosas > 16:
        raise Recusa(f"{sigilosas} opções sigilosas — o máximo é 16")

    prazo = c.get("prazo") or "15m"
    if not re.match(r"^\d{1,5}(h|m|)$", str(prazo)):
        raise Recusa("prazo inválido")
    limiar = c.get("limiar", 3)
    if not isinstance(limiar, int) or not 1 <= limiar <= 5:
        raise Recusa("limiar fora da faixa")

    args += [
        "--aptos", "aptos.txt",
        "--mesa", "mesa.txt",
        "-k", str(limiar),
        "--prazo", str(prazo),
    ]
    return rodar(args), proposta


def votar(c):
    proposta = nome(c.get("proposta"), "proposta")
    identidade = nome(c.get("identidade"), "identidade")
    opcoes = c.get("opcoes")
    if not isinstance(opcoes, list) or not opcoes:
        raise Recusa("uma opção por pergunta")
    args = ["votar", "--proposta", proposta]
    for o in opcoes:
        if not isinstance(o, str) or not o.strip():
            raise Recusa("opção vazia")
        args += ["--opcao", o.strip()]
    args += ["--identidade", identidade]
    if c.get("publico"):
        args.append("--publico")
    return rodar(args), proposta


def apurar(c):
    proposta = nome(c.get("proposta"), "proposta")
    args = ["apurar", "--proposta", proposta]
    forcar = c.get("forcar_total")
    if forcar:
        if not re.match(r"^\d{1,9}(,\d{1,9})*$", str(forcar)):
            raise Recusa("forcar-total inválido")
        args += ["--forcar-total", str(forcar)]
    return rodar(args), proposta


def verificar(c):
    proposta = nome(c.get("proposta"), "proposta")
    return rodar(["verificar", "--proposta", proposta]), proposta


def queimar(c):
    identidade = nome(c.get("identidade"), "identidade")
    return rodar(["queimar", "--identidade", identidade]), None


VERBOS = {
    "abrir": abrir,
    "votar": votar,
    "apurar": apurar,
    "verificar": verificar,
    "queimar": queimar,
}


def identidades():
    """Quem pode votar: a interseção de `aptos.txt` com as chaves da máquina."""
    try:
        aptos = [
            l.strip()
            for l in (DEMO / "aptos.txt").read_text(encoding="utf-8").splitlines()
            if l.strip()
        ]
    except OSError:
        aptos = []
    try:
        s = subprocess.run(
            ["stellar", "keys", "ls"], capture_output=True, text=True, timeout=20
        )
        chaves = {l.strip() for l in s.stdout.splitlines() if l.strip()}
    except Exception:
        chaves = set()
    return [
        {"nome": a, "tem_chave": a in chaves} for a in aptos
    ]


class Mao(BaseHTTPRequestHandler):
    server_version = "tessera-ponte"

    # O relógio bate a cada poucos segundos. Registrá-lo afoga o log justamente
    # onde se quer enxergar abrir, votar e apurar.
    SILENCIOSOS = ("/api/ledger", "/favicon.ico")

    def log_message(self, formato, *args):
        if any(q in self.path for q in self.SILENCIOSOS):
            return
        sys.stderr.write("  %s\n" % (formato % args))

    def responder(self, codigo, corpo, tipo="application/json; charset=utf-8"):
        dados = corpo if isinstance(corpo, bytes) else json.dumps(
            corpo, ensure_ascii=False
        ).encode("utf-8")
        self.send_response(codigo)
        self.send_header("Content-Type", tipo)
        self.send_header("Content-Length", str(len(dados)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(dados)

    # ---------- GET: a página, e o estado ----------

    def do_GET(self):
        caminho = self.path.split("?")[0]

        if caminho == "/api/identidades":
            return self.responder(200, {"ok": True, "identidades": identidades()})

        if caminho == "/api/estado":
            from urllib.parse import parse_qs, urlparse

            p = (parse_qs(urlparse(self.path).query).get("p") or [""])[0]
            if not NOME.match(p):
                return self.responder(400, {"ok": False, "erro": "proposta inválida"})
            d = estado(p)
            if d is None:
                return self.responder(404, {"ok": False, "erro": "não encontrada"})
            return self.responder(200, {"ok": True, "estado": d})

        if caminho == "/api/ledger":
            """O relógio da demo. A tela conta os ledgers que faltam para
            `fecha_em`, e só libera a apuração quando chegam a zero — que é o
            que o contrato vai cobrar de qualquer jeito."""
            import urllib.request
            try:
                with urllib.request.urlopen(
                    "https://horizon-testnet.stellar.org/ledgers?order=desc&limit=1",
                    timeout=10,
                ) as r:
                    v = json.load(r)
                atual = v["_embedded"]["records"][0]["sequence"]
                return self.responder(200, {"ok": True, "ledger": atual})
            except Exception:
                return self.responder(200, {"ok": False, "erro": "horizon não respondeu"})

        if caminho == "/api/propostas":
            dir_estado = DEMO / "estado"
            nomes = sorted(f.stem for f in dir_estado.glob("*.json")) if dir_estado.exists() else []
            return self.responder(200, {"ok": True, "propostas": nomes})

        # Estático, de `console/`. O charset vai explícito: sem ele os acentos
        # quebram, e isso já estragou uma gravação.
        rel = caminho.lstrip("/") or "index.html"
        alvo = (CONSOLE / rel).resolve()
        if not str(alvo).startswith(str(CONSOLE)) or not alvo.is_file():
            return self.responder(404, {"ok": False, "erro": "não encontrado"})
        tipo = TIPOS.get(alvo.suffix, "application/octet-stream")
        return self.responder(200, alvo.read_bytes(), tipo)

    # ---------- POST: os verbos ----------

    def do_POST(self):
        verbo = self.path.split("?")[0].removeprefix("/api/")
        fn = VERBOS.get(verbo)
        if not fn:
            return self.responder(404, {"ok": False, "erro": "verbo desconhecido"})
        try:
            n = int(self.headers.get("Content-Length") or 0)
            if n > 65536:
                raise Recusa("corpo grande demais")
            corpo = json.loads(self.rfile.read(n) or b"{}")
        except Recusa as e:
            return self.responder(400, {"ok": False, "erro": str(e)})
        except Exception:
            return self.responder(400, {"ok": False, "erro": "JSON inválido"})

        try:
            (codigo, saida), proposta = fn(corpo)
        except Recusa as e:
            return self.responder(400, {"ok": False, "erro": str(e)})
        except subprocess.TimeoutExpired:
            return self.responder(504, {"ok": False, "erro": "a rede não respondeu"})

        # Falha da CLI não é erro de HTTP: a recusa do contrato é um resultado
        # da demo, e a tela precisa mostrá-la na íntegra. O painel de τ existe
        # exatamente para isso.
        return self.responder(
            200,
            {
                "ok": codigo == 0,
                "saida": saida,
                "proposta": proposta,
                "estado": estado(proposta) if proposta else None,
            },
        )


def main():
    if not (DEMO / "aptos.txt").exists():
        sys.exit(f"não achei {DEMO}/aptos.txt")
    if not CONTRATO:
        print("  ⚠  TESSERA_CONTRATO não está no ambiente — `abrir` vai falhar.\n")
    try:
        s = ThreadingHTTPServer(("127.0.0.1", PORTA), Mao)
    except OSError as e:
        sys.exit(
            f"  não consegui escutar em {PORTA}: {e}\n"
            f"  outra coisa já está nessa porta. Tente:  "
            f"TESSERA_PORTA=8781 python3 console/ponte.py\n"
        )
    print(f"  TESSERA · ponte")
    print(f"  {'─' * 68}\n")
    print(f"  Tela ........ http://127.0.0.1:{PORTA}")
    print(f"  CLI ......... {BIN}")
    print(f"  Estado ...... {DEMO}/estado")
    print(f"  Contrato .... {CONTRATO or '(ausente)'}\n")
    print(f"  Cada clique vira uma transação de verdade na testnet.")
    print(f"  Ctrl-C para parar.\n")
    try:
        s.serve_forever()
    except KeyboardInterrupt:
        print("\n  parada.\n")


if __name__ == "__main__":
    main()
