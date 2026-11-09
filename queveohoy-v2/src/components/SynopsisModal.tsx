import React, { useMemo, useState } from "react";
import {
  X,
  ExternalLink,
  Play,
  Check,
  Calendar,
  Clock,
  Star,
  Users,
  Tv,
  Film,
  Plus,
  Edit3,
} from "lucide-react";
import { openUrl as open } from "@tauri-apps/plugin-opener";
import {
  MiembroReparto,
  ProveedorStreaming,
  PlataformasStreaming,
  parseCreadores,
  parseReparto,
  parsePlataformas,
} from "../utils/enrichment";

export type { MiembroReparto, ProveedorStreaming, PlataformasStreaming };

export interface SynopsisModalProps {
  isOpen: boolean;
  onClose: () => void;
  contenido: any;
  tipoTexto: string;
  generos: string[];
  localEstado: string | null;
  onPrimaryAction?: () => void;
  // Props enriquecidas (Etapa 4.1)
  posterUrl?: string;
  backdropUrl?: string;
  anio?: string;
  duracion?: string;
  puntuacion?: string;
  progreso?: {
    temp: number;
    ep: number;
    percent: number;
  } | null;
  onAddProgress?: () => void;
  onEditProgress?: () => void;
  // Props enriquecidas TMDB (Etapa 4.1.2)
  director?: string | null;
  creadores?: string[] | string | null;
  reparto?: MiembroReparto[] | string | null;
  totalTemporadas?: number | null;
  plataformas?: PlataformasStreaming | string | null;
  providersUpdatedAt?: string | null;
}

export const SynopsisModal: React.FC<SynopsisModalProps> = (props) => {
  const {
    isOpen,
    onClose,
    contenido,
    tipoTexto,
    generos,
    localEstado,
    onPrimaryAction,
    posterUrl,
    backdropUrl,
    anio,
    duracion,
    puntuacion,
    progreso,
    onAddProgress,
    onEditProgress,
    director,
    creadores,
    reparto,
    totalTemporadas,
    plataformas,
  } = props;

  const [backdropError, setBackdropError] = useState(false);
  const [posterError, setPosterError] = useState(false);

  // Normalización segura de metadatos enriquecidos
  const safeCreadores = useMemo<string[]>(() => {
    if (Array.isArray(creadores)) return creadores;
    return parseCreadores(
      typeof creadores === "string" ? creadores : contenido?.creadores
    );
  }, [creadores, contenido?.creadores]);

  const safeReparto = useMemo<MiembroReparto[]>(() => {
    if (Array.isArray(reparto)) return reparto;
    return parseReparto(
      typeof reparto === "string" ? reparto : contenido?.reparto
    );
  }, [reparto, contenido?.reparto]);

  const safePlataformas = useMemo<PlataformasStreaming>(() => {
    if (
      plataformas &&
      typeof plataformas === "object" &&
      "flatrate" in plataformas
    ) {
      return plataformas;
    }
    return parsePlataformas(
      typeof plataformas === "string" ? plataformas : contenido?.plataformas
    );
  }, [plataformas, contenido?.plataformas]);

  const safeDirector = director ?? contenido?.director ?? null;
  const safeTotalTemporadas =
    totalTemporadas ?? contenido?.total_temporadas ?? null;

  if (!isOpen || !contenido) return null;

  const isMovie = contenido.tipo === "MOVIE";
  const isEnProgreso = localEstado === "en_progreso";
  const effectiveBackdrop = !backdropError && backdropUrl ? backdropUrl : null;
  const effectivePoster = !posterError && posterUrl ? posterUrl : null;

  // External link (Letterboxd / SeriesGraph)
  const handleExternalLink = (e: React.MouseEvent) => {
    e.stopPropagation();
    const url = isMovie
      ? `https://letterboxd.com/search/${encodeURIComponent(contenido.titulo)}/`
      : `https://seriesgraph.com/search?q=${encodeURIComponent(contenido.titulo)}`;
    open(url);
  };

  // Verificación de plataformas disponibles en Argentina
  const hasFlatrate = safePlataformas.flatrate.length > 0;
  const hasRent = safePlataformas.rent.length > 0;
  const hasBuy = safePlataformas.buy.length > 0;
  const hasPlataformas = hasFlatrate || hasRent || hasBuy;

  // Helper para renderizar logo de proveedor
  const renderProvider = (p: ProveedorStreaming) => {
    const logoUrl = p.logo_path
      ? p.logo_path.startsWith("http")
        ? p.logo_path
        : `https://image.tmdb.org/t/p/w92${p.logo_path.startsWith("/") ? p.logo_path : `/${p.logo_path}`}`
      : null;

    return (
      <div
        key={`${p.tipo}-${p.id}-${p.nombre}`}
        className="flex items-center gap-1.5 px-2.5 py-1 rounded-xl bg-white/[0.05] hover:bg-white/[0.09] border border-white/10 transition-colors shadow-sm"
        title={p.nombre}
      >
        {logoUrl ? (
          <img
            src={logoUrl}
            alt={p.nombre}
            className="w-5 h-5 rounded-md object-cover bg-black/40"
            onError={(e) => {
              e.currentTarget.style.display = "none";
            }}
          />
        ) : (
          <Tv size={14} className="text-violet-400" />
        )}
        <span className="text-xs font-medium text-zinc-200">{p.nombre}</span>
      </div>
    );
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 md:p-6 bg-black/80 backdrop-blur-md animate-in fade-in duration-200"
      onClick={onClose}
    >
      <div
        className="max-w-3xl w-full bg-[#11121D] border border-white/10 rounded-3xl shadow-2xl shadow-black/90 overflow-hidden relative flex flex-col max-h-[92vh] sm:max-h-[88vh] animate-in fade-in zoom-in-95 slide-in-from-bottom-2 duration-200 ease-out select-none"
        onClick={(e) => e.stopPropagation()}
      >
        {/* ======================================================== */}
        {/* A. BACKDROP CINEMATOGRÁFICO Y ENCABEZADO                 */}
        {/* ======================================================== */}
        <div className="relative w-full h-48 sm:h-56 md:h-64 shrink-0 overflow-hidden bg-neutral-950">
          {effectiveBackdrop ? (
            <img
              src={effectiveBackdrop}
              alt=""
              className="w-full h-full object-cover object-center"
              onError={() => setBackdropError(true)}
            />
          ) : (
            // Fallback elegante si no hay backdrop: poster desenfocado o gradiente de cine
            <div className="w-full h-full relative overflow-hidden bg-gradient-to-br from-violet-950/40 via-[#11121D] to-[#0A0A10]">
              {effectivePoster && (
                <img
                  src={effectivePoster}
                  alt=""
                  className="w-full h-full object-cover blur-2xl scale-125 opacity-25"
                />
              )}
            </div>
          )}

          {/* Gradientes cinematográficos de fundición hacia el cuerpo y bordes */}
          <div className="absolute inset-0 bg-gradient-to-t from-[#11121D] via-[#11121D]/60 to-transparent pointer-events-none" />
          <div className="absolute inset-0 bg-gradient-to-r from-[#11121D]/80 via-transparent to-black/40 pointer-events-none" />
          <div className="absolute top-0 inset-x-0 h-20 bg-gradient-to-b from-black/70 to-transparent pointer-events-none" />

          {/* Botón de cierre en la esquina superior derecha */}
          <button
            onClick={onClose}
            className="absolute top-3.5 right-3.5 sm:top-4 sm:right-4 z-30 p-2 sm:p-2.5 text-zinc-300 hover:text-white bg-black/60 hover:bg-black/85 backdrop-blur-md rounded-full border border-white/15 transition-all cursor-pointer shadow-lg active:scale-95"
            title="Cerrar modal"
          >
            <X size={18} />
          </button>
        </div>

        {/* ======================================================== */}
        {/* B. PÓSTER, TÍTULO Y METADATA INTEGRADA                   */}
        {/* ======================================================== */}
        <div className="-mt-16 sm:-mt-20 md:-mt-24 relative z-20 px-5 sm:px-8 flex flex-col sm:flex-row items-center sm:items-end gap-4 sm:gap-6 shrink-0">
          {/* Póster vertical con sombra cinematográfica */}
          <div className="w-28 sm:w-32 md:w-36 aspect-[2/3] shrink-0 rounded-2xl overflow-hidden shadow-2xl shadow-black/90 border border-white/15 bg-neutral-900 relative">
            {effectivePoster ? (
              <img
                src={effectivePoster}
                alt={contenido.titulo}
                className="w-full h-full object-cover"
                onError={() => setPosterError(true)}
              />
            ) : (
              <div className="w-full h-full flex flex-col items-center justify-center p-3 text-center bg-violet-950/30">
                <Film size={28} className="text-violet-400 mb-1 opacity-70" />
                <span className="text-[10px] text-zinc-400 line-clamp-2">
                  {contenido.titulo}
                </span>
              </div>
            )}
          </div>

          {/* Título, Ficha Técnica y Enlace Externo */}
          <div className="flex-1 min-w-0 text-center sm:text-left pt-1 sm:pt-0">
            <h2 className="text-xl sm:text-2xl md:text-3xl font-extrabold text-white tracking-tight leading-tight line-clamp-2 drop-shadow-md">
              {contenido.titulo}
            </h2>

            {contenido.titulo_original &&
              contenido.titulo_original !== contenido.titulo && (
                <p className="text-xs sm:text-sm text-zinc-400 italic line-clamp-1 mt-0.5">
                  {contenido.titulo_original}
                </p>
              )}

            {/* Badges y chips de ficha técnica */}
            <div className="flex flex-wrap items-center justify-center sm:justify-start gap-2 mt-2.5 text-xs text-zinc-300">
              {/* Badge Tipo */}
              <span className="bg-violet-950/80 border border-violet-500/40 text-violet-300 font-semibold px-2.5 py-0.5 rounded-full text-[11px] uppercase tracking-wider">
                {tipoTexto}
              </span>

              {/* Año de estreno */}
              {anio && (
                <span className="flex items-center gap-1 font-medium text-zinc-300">
                  <Calendar size={12} className="text-zinc-400" />
                  <span>{anio}</span>
                </span>
              )}

              {/* Duración (películas) */}
              {isMovie && duracion && (
                <>
                  <span className="text-zinc-600 text-[10px] hidden sm:inline">•</span>
                  <span className="flex items-center gap-1 text-zinc-300">
                    <Clock size={12} className="text-zinc-400" />
                    <span>{duracion}</span>
                  </span>
                </>
              )}

              {/* Cantidad de temporadas (series) */}
              {!isMovie && (safeTotalTemporadas || duracion) && (
                <>
                  <span className="text-zinc-600 text-[10px] hidden sm:inline">•</span>
                  <span className="flex items-center gap-1 text-zinc-300">
                    <Tv size={12} className="text-zinc-400" />
                    <span>
                      {safeTotalTemporadas
                        ? `${safeTotalTemporadas} temporada${safeTotalTemporadas > 1 ? "s" : ""}`
                        : duracion}
                    </span>
                  </span>
                </>
              )}

              {/* Puntuación */}
              {puntuacion && puntuacion !== "0" && (
                <>
                  <span className="text-zinc-600 text-[10px] hidden sm:inline">•</span>
                  <span className="flex items-center gap-1 font-semibold text-amber-400">
                    <Star size={12} className="fill-amber-400 text-amber-400" />
                    <span>{puntuacion}</span>
                  </span>
                </>
              )}

              {/* Enlace externo a Letterboxd / SeriesGraph */}
              <button
                onClick={handleExternalLink}
                className="inline-flex items-center gap-1.5 px-3 py-0.5 rounded-full text-xs font-medium text-zinc-300 hover:text-white bg-white/5 hover:bg-white/10 border border-white/10 hover:border-violet-400/40 transition-all cursor-pointer shadow-sm ml-auto sm:ml-1"
                title={`Buscar en ${isMovie ? "Letterboxd" : "SeriesGraph"}`}
              >
                <ExternalLink size={12} className="text-violet-400" />
                <span>{isMovie ? "Letterboxd" : "SeriesGraph"}</span>
              </button>
            </div>

            {/* Chips de géneros */}
            {generos && generos.length > 0 && (
              <div className="flex flex-wrap items-center justify-center sm:justify-start gap-1.5 mt-2.5">
                {generos.map((g) => (
                  <span
                    key={g}
                    className="px-2.5 py-0.5 bg-neutral-900/80 border border-white/10 rounded-full text-[11px] font-medium text-purple-200/90 shadow-sm"
                  >
                    {g}
                  </span>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Separador sutil */}
        <div className="w-full h-px bg-white/10 mt-4 shrink-0" />

        {/* ======================================================== */}
        {/* C. CUERPO SCROLLABLE CON METADATA Y DETALLES             */}
        {/* ======================================================== */}
        <div className="px-5 sm:px-8 py-4 sm:py-5 space-y-5 overflow-y-auto overflow-x-hidden flex-1 scrollbar-thin scrollbar-thumb-white/15 scrollbar-track-transparent">
          {/* C.1 Barra de progreso de serie (si está en progreso) */}
          {!isMovie && isEnProgreso && progreso && (
            <div className="bg-violet-950/30 border border-violet-500/20 rounded-2xl p-3.5 space-y-2">
              <div className="flex justify-between items-center text-xs">
                <span className="font-semibold text-violet-300 flex items-center gap-1.5">
                  <span>Temporada {progreso.temp} · Episodio {progreso.ep}</span>
                  {onEditProgress && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onEditProgress();
                      }}
                      className="p-1 text-zinc-400 hover:text-violet-300 hover:bg-white/10 rounded transition-colors cursor-pointer"
                      title="Editar progreso manualmente"
                    >
                      <Edit3 size={12} className="text-violet-400 hover:text-violet-300" />
                    </button>
                  )}
                </span>
                <span className="text-zinc-400 font-medium">
                  {progreso.percent}% completado
                </span>
              </div>
              <div className="w-full h-2 bg-black/40 rounded-full overflow-hidden border border-white/5">
                <div
                  className="h-full rounded-full bg-gradient-to-r from-violet-500 to-purple-500 transition-all duration-500"
                  style={{ width: `${Math.min(100, Math.max(0, progreso.percent))}%` }}
                />
              </div>
            </div>
          )}

          {/* C.2 Dirección / Creación */}
          {isMovie && safeDirector && (
            <div className="flex items-baseline gap-2 text-xs">
              <span className="text-zinc-400 uppercase tracking-wider font-semibold shrink-0">
                Dirección:
              </span>
              <span className="text-zinc-200 font-medium">{safeDirector}</span>
            </div>
          )}

          {!isMovie && safeCreadores.length > 0 && (
            <div className="flex items-baseline gap-2 text-xs flex-wrap">
              <span className="text-zinc-400 uppercase tracking-wider font-semibold shrink-0">
                {safeCreadores.length > 1 ? "Creadores:" : "Creador:"}
              </span>
              <span className="text-zinc-200 font-medium">
                {safeCreadores.join(", ")}
              </span>
            </div>
          )}

          {/* C.3 Sinopsis */}
          <div>
            <h4 className="text-xs uppercase font-semibold text-zinc-400 tracking-wider mb-1.5">
              Sinopsis
            </h4>
            <p className="text-zinc-300 leading-relaxed text-sm md:text-[15px] select-text">
              {contenido.sinopsis ||
                contenido.descripcion ||
                "Sin descripción disponible."}
            </p>
          </div>

          {/* C.4 Reparto Principal (Cast) */}
          {safeReparto.length > 0 && (
            <div>
              <h4 className="text-xs uppercase font-semibold text-zinc-400 tracking-wider mb-2.5 flex items-center gap-1.5">
                <Users size={14} className="text-violet-400" />
                <span>Reparto Principal</span>
              </h4>
              <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-2">
                {safeReparto.map((actor, idx) => (
                  <div
                    key={`${actor.nombre}-${idx}`}
                    className="bg-white/[0.03] hover:bg-white/[0.06] border border-white/5 rounded-xl p-2.5 transition-colors"
                  >
                    <p
                      className="text-xs font-semibold text-zinc-100 truncate"
                      title={actor.nombre}
                    >
                      {actor.nombre}
                    </p>
                    <p
                      className="text-[11px] text-zinc-400 truncate mt-0.5"
                      title={actor.personaje}
                    >
                      {actor.personaje || "—"}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* C.5 Plataformas de streaming en Argentina */}
          {hasPlataformas && (
            <div>
              <h4 className="text-xs uppercase font-semibold text-zinc-400 tracking-wider mb-3 flex items-center gap-1.5">
                <Tv size={14} className="text-violet-400" />
                <span>Disponible en Argentina</span>
              </h4>
              <div className="space-y-3">
                {/* Suscripción (flatrate) */}
                {hasFlatrate && (
                  <div>
                    <span className="text-[10px] font-semibold text-zinc-400 uppercase tracking-wider block mb-1.5">
                      Suscripción
                    </span>
                    <div className="flex flex-wrap items-center gap-2">
                      {safePlataformas.flatrate.map(renderProvider)}
                    </div>
                  </div>
                )}

                {/* Alquiler (rent) */}
                {hasRent && (
                  <div>
                    <span className="text-[10px] font-semibold text-zinc-400 uppercase tracking-wider block mb-1.5">
                      Alquiler
                    </span>
                    <div className="flex flex-wrap items-center gap-2">
                      {safePlataformas.rent.map(renderProvider)}
                    </div>
                  </div>
                )}

                {/* Compra (buy) */}
                {hasBuy && (
                  <div>
                    <span className="text-[10px] font-semibold text-zinc-400 uppercase tracking-wider block mb-1.5">
                      Compra
                    </span>
                    <div className="flex flex-wrap items-center gap-2">
                      {safePlataformas.buy.map(renderProvider)}
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        {/* ======================================================== */}
        {/* D. ACCIONES INFERIORES FIJAS                            */}
        {/* ======================================================== */}
        <div className="border-t border-white/10 bg-[#11121D]/95 backdrop-blur-md px-5 sm:px-8 py-3.5 sm:py-4 flex flex-col sm:flex-row items-center justify-between gap-3 shrink-0">
          <div className="flex items-center gap-2 w-full sm:w-auto">
            {/* Si es serie en progreso y cuenta con onAddProgress */}
            {!isMovie && isEnProgreso && onAddProgress && (
              <button
                onClick={onAddProgress}
                className="w-full sm:w-auto px-5 py-2.5 rounded-xl font-bold flex items-center justify-center gap-2 bg-gradient-to-r from-violet-600 to-purple-600 hover:from-violet-500 hover:to-purple-500 shadow-lg shadow-purple-950/40 active:scale-[0.98] transition-all text-white text-sm cursor-pointer border border-violet-400/30"
              >
                <Plus size={16} />
                <span>+1 Episodio</span>
              </button>
            )}

            {/* Acción principal si no está en progreso */}
            {!isEnProgreso && onPrimaryAction && (
              <button
                onClick={() => {
                  onClose();
                  onPrimaryAction();
                }}
                className="w-full sm:w-auto px-6 py-2.5 rounded-xl font-bold flex items-center justify-center gap-2 bg-gradient-to-r from-violet-600 to-purple-600 hover:from-violet-500 hover:to-purple-500 shadow-lg shadow-purple-950/40 active:scale-[0.98] transition-all text-white text-sm cursor-pointer border border-violet-400/30"
              >
                {isMovie ? (
                  <>
                    <Check size={16} />
                    <span>Marcar como vista</span>
                  </>
                ) : (
                  <>
                    <Play size={16} fill="currentColor" />
                    <span>Empezar a ver</span>
                  </>
                )}
              </button>
            )}
          </div>

          {/* Botón secundario para cerrar */}
          <button
            onClick={onClose}
            className="w-full sm:w-auto px-4 py-2 rounded-xl text-xs font-semibold text-zinc-400 hover:text-white bg-white/[0.04] hover:bg-white/[0.08] border border-white/10 transition-colors cursor-pointer text-center"
          >
            Cerrar ficha
          </button>
        </div>
      </div>
    </div>
  );
};

export default SynopsisModal;
