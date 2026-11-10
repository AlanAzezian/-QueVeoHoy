import React from "react";
import { Play, Check, Clock, ArrowRight } from "lucide-react";

interface ActionBarProps {
  isEnProgreso: boolean;
  isMovie: boolean;
  onPrimary: () => void;
  onMarkWatched: () => void;
  onSave: () => void;
  onNext: () => void;
  disabled?: boolean;
}

export const ActionBar: React.FC<ActionBarProps> = ({
  isEnProgreso,
  isMovie,
  onPrimary,
  onMarkWatched,
  onSave,
  onNext,
  disabled = false
}) => {
  const isSeriesProgress = isEnProgreso && !isMovie;
  const primaryLabel = isSeriesProgress ? "+1 Episodio" : "Empezar a ver";

  return (
    <div className="flex items-center justify-center gap-2 sm:gap-2.5 w-full whitespace-nowrap mt-1 select-none flex-nowrap">
      {/* Botón Primario Destacado (Gradiente Violeta / Púrpura) */}
      <button
        onClick={onPrimary}
        disabled={disabled}
        className="bg-gradient-to-r from-violet-600 via-violet-500 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 disabled:opacity-40 disabled:cursor-not-allowed text-white font-semibold px-5 py-2 rounded-xl shadow-lg shadow-violet-950/50 text-xs transition-colors duration-200 flex items-center gap-2 cursor-pointer shrink-0 border border-violet-400/30"
      >
        <Play size={13} fill="currentColor" />
        <span>{primaryLabel}</span>
      </button>

      {/* Botón Secundario 1: Ya la vi */}
      <button
        onClick={onMarkWatched}
        disabled={disabled}
        className="group bg-neutral-950/80 hover:bg-neutral-900 backdrop-blur-md disabled:opacity-40 disabled:cursor-not-allowed border border-white/15 hover:border-emerald-500/30 text-zinc-200 hover:text-white px-3.5 py-2 rounded-xl text-xs transition-colors duration-200 flex items-center gap-1.5 cursor-pointer shrink-0 shadow-md [text-shadow:_0_1px_2px_rgba(0,0,0,0.8)]"
      >
        <Check size={13} className="text-zinc-400 group-hover:text-emerald-400 transition-colors drop-shadow-sm" />
        <span>Ya la vi</span>
      </button>

      {/* Botón Secundario 2: Guardar */}
      <button
        onClick={onSave}
        disabled={disabled}
        className="group bg-neutral-950/80 hover:bg-neutral-900 backdrop-blur-md disabled:opacity-40 disabled:cursor-not-allowed border border-white/15 hover:border-amber-500/30 text-zinc-200 hover:text-white px-3.5 py-2 rounded-xl text-xs transition-colors duration-200 flex items-center gap-1.5 cursor-pointer shrink-0 shadow-md [text-shadow:_0_1px_2px_rgba(0,0,0,0.8)]"
      >
        <Clock size={13} className="text-zinc-400 group-hover:text-amber-400 transition-colors drop-shadow-sm" />
        <span>Guardar</span>
      </button>

      {/* Botón Secundario 3: Siguiente */}
      <button
        onClick={onNext}
        disabled={disabled}
        className="group bg-neutral-950/80 hover:bg-neutral-900 backdrop-blur-md disabled:opacity-40 disabled:cursor-not-allowed border border-white/15 hover:border-violet-500/30 text-zinc-200 hover:text-white px-3.5 py-2 rounded-xl text-xs transition-colors duration-200 flex items-center gap-1.5 cursor-pointer shrink-0 shadow-md [text-shadow:_0_1px_2px_rgba(0,0,0,0.8)]"
      >
        <ArrowRight size={13} className="text-zinc-400 group-hover:text-violet-400 transition-colors drop-shadow-sm" />
        <span>Siguiente</span>
      </button>
    </div>
  );
};

export default ActionBar;
