/**
 * Os componentes que o dapp repete.
 *
 * Todos nasceram dentro de `paginas/Votar.tsx`, quando só a cédula tinha
 * desenho. Nenhum deles é da cédula: o ícone, o estado central, a trilha de
 * etapas e a lista de passos de um envio valem para abrir, comparecer, votar e
 * apurar igual. Ficaram aqui para não serem copiados cinco vezes.
 */
import type { ReactNode } from "react";

export type IconeNome =
  | "lock" | "check" | "arrow" | "shield" | "users" | "clock"
  | "plus" | "external" | "key" | "scale" | "eye-off" | "terminal";

export function Icone({ nome, className = "" }: { nome: IconeNome; className?: string }) {
  return <svg className={`icone ${className}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {nome === "lock" && <><rect x="5" y="10" width="14" height="11" rx="2" /><path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v3" /></>}
    {nome === "check" && <path d="m5 12 4 4L19 6" />}
    {nome === "arrow" && <path d="M4 12h16m-6-6 6 6-6 6" />}
    {nome === "shield" && <><path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6z" /><path d="m8 12 3 3 5-6" /></>}
    {nome === "users" && <><circle cx="9" cy="8" r="3.2" /><path d="M3 20a6 6 0 0 1 12 0M16.5 5.6a3.2 3.2 0 0 1 0 5.8M18 14.4a6 6 0 0 1 3 5.6" /></>}
    {nome === "clock" && <><circle cx="12" cy="12" r="8.6" /><path d="M12 7v5.3l3.3 2" /></>}
    {nome === "plus" && <path d="M12 5v14M5 12h14" />}
    {nome === "external" && <><path d="M14 4h6v6" /><path d="M20 4 11 13" /><path d="M18 14v5a1.6 1.6 0 0 1-1.6 1.6H5.6A1.6 1.6 0 0 1 4 19V8.2A1.6 1.6 0 0 1 5.6 6.6H10" /></>}
    {nome === "key" && <><circle cx="8" cy="12" r="4" /><path d="M12 12h9M18 12v3.4M15.2 12v2.4" /></>}
    {nome === "scale" && <><path d="M12 4v16M6 8h12M4.5 8 2 14h5zM19.5 8 17 14h5z" /><path d="M8 20h8" /></>}
    {nome === "eye-off" && <><path d="M3 3l18 18" /><path d="M10.6 6.3A8.6 8.6 0 0 1 12 6.2c5 0 9 5.8 9 5.8a16 16 0 0 1-3 3.5M6.4 7.9A16 16 0 0 0 3 12s4 5.8 9 5.8a8.4 8.4 0 0 0 3.2-.6" /><path d="M9.9 10.2a3 3 0 0 0 4.1 4.2" /></>}
    {nome === "terminal" && <><rect x="3" y="4.5" width="18" height="15" rx="2" /><path d="m7.5 10 2.6 2.2-2.6 2.2M13 15h4" /></>}
  </svg>;
}

/** Carregando, vazio, erro, sucesso: o mesmo bloco centrado. */
export function Estado({ titulo, children, curto = false }: { titulo: string; children?: ReactNode; curto?: boolean }) {
  return <div className={`ballot-state${curto ? " ballot-state--curto" : ""} stage-enter`} role="status">
    <h2>{titulo}</h2>
    {children}
  </div>;
}

export function Carregando({ titulo, nota }: { titulo: string; nota?: string }) {
  return <div className="ballot-state ballot-state--curto" role="status">
    <span className="loading-ring" />
    <h2>{titulo}</h2>
    {nota && <p>{nota}</p>}
  </div>;
}

export function Aviso({ tipo = "nota", children }: { tipo?: "erro" | "nota"; children: ReactNode }) {
  if (!children) return null;
  return <p className={tipo === "erro" ? "error-message" : "notice-message"} role={tipo === "erro" ? "alert" : "status"}>{children}</p>;
}

/**
 * Os passos de um envio, em linguagem de pessoa.
 *
 * O detalhe técnico do mesmo envio vive em `/bastidores`. Aqui ficam só as três
 * ou quatro frases que dizem a quem está esperando que nada travou.
 */
export function Passos({ passos, atual }: { passos: string[]; atual: number }) {
  return <ol className="sending-steps" aria-live="polite">
    {passos.map((s, i) => <li key={s} className={i <= atual ? "reached" : ""}>
      <span>{i < atual ? <Icone nome="check" /> : i === atual ? <span className="status-dot" /> : `0${i + 1}`}</span>
      {s}{i === atual && <span className="sr-only">, em andamento</span>}
    </li>)}
  </ol>;
}

export type Etapa = { label: string; completo: boolean; ativo: boolean };

/** Presença · Voto · Confirmação — a mesma trilha em `/comparecer` e `/votar`. */
export function Trilha({ etapas }: { etapas: Etapa[] }) {
  return <ol className="voting-steps" aria-label="Etapas da participação">
    {etapas.map((s, i) => <li key={s.label} className={`${s.completo ? "complete" : ""} ${s.ativo ? "current" : ""}`} aria-current={s.ativo ? "step" : undefined}>
      <span className="step-number">{s.completo ? <Icone nome="check" /> : `0${i + 1}`}</span>
      <span>{s.label}</span>
      {i < etapas.length - 1 && <span className="step-connector" aria-hidden="true" />}
    </li>)}
  </ol>;
}

/**
 * O diário não mora mais no pé das páginas: ele tem rota própria, para caber
 * numa segunda janela ao lado desta.
 */
export function LinkBastidores({ origem }: { origem?: string }) {
  return <a className="link-bastidores" href={origem ? `/bastidores?origem=${origem}` : "/bastidores"} target="_blank" rel="noopener noreferrer">
    <Icone nome="terminal" /> VER NOS BASTIDORES <span aria-hidden="true">↗</span>
  </a>;
}
