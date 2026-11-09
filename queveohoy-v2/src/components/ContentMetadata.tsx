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
    <div className="w-full max-w-xl mx-auto flex flex-col items-center text-center gap-1 select-none">
      {/* 1. Título principal destacado */}
      <h2 className="text-xl sm:text-2xl font-bold text-white tracking-tight line-clamp-1 px-4 drop-shadow-sm">
        {titulo || "Cargando..."}
      </h2>

      {/* 2. Ficha técnica integrada (Tipo, Año, Duración / Temporadas, Puntuación) */}
      <div className="text-xs text-zinc-300 flex items-center justify-center gap-2.5 flex-wrap">
        {tipo && (
          <span className="bg-violet-950/70 border border-violet-500/30 text-violet-300 text-[10px] font-semibold px-2 py-0.5 rounded-full uppercase tracking-wider">
            {tipo}
          </span>
        )}
        {anio && (
          <span className="flex items-center gap-1 text-zinc-300">
            <Calendar size={12} className="text-zinc-400" />
            <span>{anio}</span>
          </span>
        )}
        {duracion && (
          <>
            <span className="text-zinc-600 text-[10px]">•</span>
            <span className="flex items-center gap-1 text-zinc-300">
              <Clock size={12} className="text-zinc-400" />
              <span>{duracion}</span>
            </span>
          </>
        )}
        {temporadas && !duracion && (
          <>
            <span className="text-zinc-600 text-[10px]">•</span>
            <span className="flex items-center gap-1 text-zinc-300">
              <Tv size={12} className="text-zinc-400" />
              <span>{temporadas}</span>
            </span>
          </>
        )}
        {puntuacion && puntuacion !== "0" && (
          <>
            <span className="text-zinc-600 text-[10px]">•</span>
            <span className="flex items-center gap-1 text-amber-400 font-medium">
              <Star size={12} className="fill-amber-400 text-amber-400" />
              <span>{puntuacion}</span>
            </span>
          </>
        )}
      </div>

      {/* 3. Chips de géneros limpios y alineados */}
      {visibleGeneros.length > 0 && (
        <div className="flex flex-wrap items-center justify-center gap-1.5 mt-0.5">
          {visibleGeneros.map((g) => (
            <span
              key={g}
              className="bg-neutral-900/80 border border-white/10 px-2.5 py-0.5 rounded-full text-[11px] font-medium text-purple-200/90 shadow-sm"
            >
              {g}
            </span>
          ))}
        </div>
      )}

      {/* 4. Barra de progreso (condicional: solo series en progreso) */}
      {showProgress && progreso && (
        <div className="w-full max-w-sm flex flex-col items-center my-0.5 px-2">
          <div className="h-1.5 w-full bg-white/10 rounded-full overflow-hidden my-0.5">
            <div
              className="h-full rounded-full bg-gradient-to-r from-violet-500 to-purple-500 transition-all duration-500 ease-out"
              style={{ width: `${progreso.percent}%` }}
            />
          </div>
          <div className="flex justify-between items-center w-full text-[10px] text-zinc-400 mt-0.5 px-1">
            <span className="flex items-center gap-1.5">
              <span>T{progreso.temp} · Cap {progreso.ep}</span>
              {onEditProgress && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onEditProgress();
                  }}
                  className="p-0.5 text-zinc-400 hover:text-violet-300 hover:bg-white/10 rounded transition-colors cursor-pointer"
                  title="Editar progreso manualmente"
                >
                  <Edit3 size={11} className="text-violet-400 hover:text-violet-300" />
                </button>
              )}
            </span>
            <span>{progreso.percent}% completado</span>
          </div>
        </div>
      )}

      {/* 5. Sinopsis concisa en 2 líneas */}
      {sinopsis && sinopsis !== "Sin descripción disponible." && (
        <p className="text-zinc-400 text-xs leading-relaxed line-clamp-2 px-3 mt-0.5 max-w-lg">
          {sinopsis}
        </p>
      )}
    </div>
  );
};

export default ContentMetadata;
