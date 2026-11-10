import React from "react";
import { Calendar, Clock, Star, Edit3, Tv } from "lucide-react";

interface ContentMetadataProps {
  titulo: string;
  anio: string;
  duracion: string;
  temporadas?: string;
  puntuacion?: string;
  generos: string[];
  sinopsis: string;
  isEnProgreso?: boolean;
  tipo?: string;
  progreso?: {
    temp: number;
    ep: number;
    percent: number;
  } | null;
  onEditProgress?: () => void;
}

export const ContentMetadata: React.FC<ContentMetadataProps> = ({
  titulo,
  anio,
  duracion,
  temporadas,
  puntuacion,
  generos,
  sinopsis,
  isEnProgreso = false,
  tipo,
  progreso,
  onEditProgress
}) => {
  const isSeries = tipo !== "MOVIE";
  const showProgress = isEnProgreso && isSeries && Boolean(progreso);

  // Filtrar y limitar los géneros a máximo 4 para una presentación equilibrada
  const visibleGeneros = generos && Array.isArray(generos) ? generos.filter(Boolean).slice(0, 4) : [];

  return (
    <div className="relative w-full max-w-xl mx-auto flex flex-col items-center text-center gap-1 select-none">
      {/* Sutil halo ambiental difuso detrás del bloque de metadata para asegurar contraste sin crear una tarjeta rígida */}
      <div
        className="absolute inset-0 -inset-x-8 -inset-y-4 bg-[radial-gradient(ellipse_at_center,rgba(8,9,16,0.65)_0%,rgba(8,9,16,0.35)_60%,transparent_100%)] rounded-3xl -z-10 pointer-events-none blur-lg"
        aria-hidden="true"
      />

      {/* 1. Título principal destacado con separación inferior refinada */}
      <h2 className="text-xl sm:text-2xl font-bold text-white tracking-tight line-clamp-1 px-4 mb-1.5 drop-shadow-[0_2px_8px_rgba(0,0,0,0.95)] [text-shadow:_0_2px_12px_rgba(0,0,0,0.95),_0_1px_3px_rgba(0,0,0,0.9)]">
        {titulo || "Cargando..."}
      </h2>

      {/* 2. Ficha técnica integrada (Tipo, Año, Duración / Temporadas, Puntuación) con tipografía y separación optimizadas */}
      <div className="text-[13px] text-zinc-200 flex items-center justify-center gap-3 sm:gap-3.5 flex-wrap drop-shadow-[0_1px_4px_rgba(0,0,0,0.9)]">
        {tipo && (
          <span className="bg-violet-950/80 border border-violet-500/40 text-violet-200 text-[10.5px] font-semibold px-2.5 py-0.5 rounded-full uppercase tracking-wider backdrop-blur-md shadow-md [text-shadow:_0_1px_2px_rgba(0,0,0,0.8)]">
            {tipo}
          </span>
        )}
        {anio && (
          <span className="flex items-center gap-1.5 text-zinc-200 font-medium [text-shadow:_0_1px_3px_rgba(0,0,0,0.9)]">
            <Calendar size={13} className="text-violet-300/90 drop-shadow-sm" />
            <span>{anio}</span>
          </span>
        )}
        {duracion && (
          <>
            <span className="text-zinc-400/80 text-[11px] [text-shadow:_0_1px_2px_rgba(0,0,0,0.9)]">•</span>
            <span className="flex items-center gap-1.5 text-zinc-200 font-medium [text-shadow:_0_1px_3px_rgba(0,0,0,0.9)]">
              <Clock size={13} className="text-violet-300/90 drop-shadow-sm" />
              <span>{duracion}</span>
            </span>
          </>
        )}
        {temporadas && !duracion && (
          <>
            <span className="text-zinc-400/80 text-[11px] [text-shadow:_0_1px_2px_rgba(0,0,0,0.9)]">•</span>
            <span className="flex items-center gap-1.5 text-zinc-200 font-medium [text-shadow:_0_1px_3px_rgba(0,0,0,0.9)]">
              <Tv size={13} className="text-violet-300/90 drop-shadow-sm" />
              <span>{temporadas}</span>
            </span>
          </>
        )}
        {puntuacion && puntuacion !== "0" && (
          <>
            <span className="text-zinc-400/80 text-[11px] [text-shadow:_0_1px_2px_rgba(0,0,0,0.9)]">•</span>
            <span className="inline-flex items-center gap-1 bg-amber-950/50 border border-amber-500/40 text-amber-300 text-xs font-semibold px-2.5 py-0.5 rounded-full backdrop-blur-md shadow-sm [text-shadow:_0_1px_2px_rgba(0,0,0,0.9)]">
              <Star size={12} className="fill-amber-400 text-amber-400 drop-shadow-sm" />
              <span>{puntuacion}</span>
            </span>
          </>
        )}
      </div>

      {/* 3. Chips de géneros limpios y alineados con separación refinada */}
      {visibleGeneros.length > 0 && (
        <div className="flex flex-wrap items-center justify-center gap-1.5 mt-1.5">
          {visibleGeneros.map((g) => (
            <span
              key={g}
              className="bg-neutral-950/80 border border-white/15 px-2.5 py-0.5 rounded-full text-[11px] font-medium text-purple-200 shadow-md backdrop-blur-md [text-shadow:_0_1px_2px_rgba(0,0,0,0.8)]"
            >
              {g}
            </span>
          ))}
        </div>
      )}

      {/* 4. Barra de progreso (condicional: solo series en progreso) */}
      {showProgress && progreso && (
        <div className="w-full max-w-sm flex flex-col items-center my-1 px-2">
          <div className="h-2 w-full bg-black/70 border border-white/15 rounded-full overflow-hidden my-0.5 shadow-inner backdrop-blur-sm p-[1px]">
            <div
              className="h-full rounded-full bg-gradient-to-r from-violet-500 via-violet-400 to-fuchsia-400 shadow-[0_0_8px_rgba(168,85,247,0.6)] transition-all duration-500 ease-out"
              style={{ width: `${progreso.percent}%` }}
            />
          </div>
          <div className="flex justify-between items-center w-full text-[11px] font-medium text-zinc-200 mt-0.5 px-1 [text-shadow:_0_1px_3px_rgba(0,0,0,0.9)]">
            <span className="flex items-center gap-1.5">
              <span>T{progreso.temp} · Cap {progreso.ep}</span>
              {onEditProgress && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onEditProgress();
                  }}
                  className="p-0.5 text-zinc-300 hover:text-violet-200 hover:bg-white/10 rounded transition-colors cursor-pointer"
                  title="Editar progreso manualmente"
                >
                  <Edit3 size={11} className="text-violet-400 hover:text-violet-300 drop-shadow-sm" />
                </button>
              )}
            </span>
            <span className="text-violet-300 font-semibold">{progreso.percent}% completado</span>
          </div>
        </div>
      )}

      {/* 5. Sinopsis concisa en 2 líneas, secundaria y discreta */}
      {sinopsis && sinopsis !== "Sin descripción disponible." && (
        <p className="text-zinc-200/90 text-xs leading-relaxed line-clamp-2 px-3 mt-1.5 max-w-lg font-normal [text-shadow:_0_1px_4px_rgba(0,0,0,0.95),_0_1px_2px_rgba(0,0,0,0.9)]">
          {sinopsis}
        </p>
      )}
    </div>
  );
};

export default ContentMetadata;
