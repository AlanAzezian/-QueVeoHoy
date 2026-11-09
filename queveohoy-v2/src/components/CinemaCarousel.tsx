import React, { useState, useEffect, useRef } from "react";
import { ChevronLeft, ChevronRight, Loader2 } from "lucide-react";
import { RouletteCarousel3D, RouletteItem } from "./RouletteCarousel3D";

interface CinemaCarouselProps {
  hasPrevious: boolean;
  leftCard: React.ReactNode;
  centerCard: React.ReactNode;
  rightCard: React.ReactNode;
  onAnterior?: () => void;
  onSiguiente?: () => void;
  activeId?: string | number;
  isSpinning?: boolean;
  currentContent?: RouletteItem | null;
  nextContent?: RouletteItem | null;
  poolItems?: RouletteItem[];
  onSpinComplete?: () => void;
}

export const CinemaCarousel: React.FC<CinemaCarouselProps> = ({
  hasPrevious,
  leftCard,
  centerCard,
  rightCard,
  onAnterior,
  onSiguiente,
  activeId,
  isSpinning = false,
  currentContent = null,
  nextContent = null,
  poolItems = [],
  onSpinComplete = () => {}
}) => {
  const [navDirection, setNavDirection] = useState<"next" | "prev" | null>(null);
  const [isLocked, setIsLocked] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const prevActiveIdRef = useRef<string | number | undefined>(activeId);

  useEffect(() => {
    prevActiveIdRef.current = activeId;
  }, [activeId]);

  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  const triggerAnimation = (dir: "next" | "prev", callback?: () => void) => {
    if (isLocked || isSpinning) return;
    setIsLocked(true);
    setNavDirection(dir);
    callback?.();

    if (timerRef.current) clearTimeout(timerRef.current);
    const duration = dir === "next" ? 500 : 420;
    timerRef.current = setTimeout(() => {
      setNavDirection(null);
      setIsLocked(false);
    }, duration);
  };

  const handleSiguiente = () => {
    if (!onSiguiente || isLocked || isSpinning) return;
    onSiguiente();
  };

  const handleAnterior = () => {
    if (!hasPrevious || !onAnterior || isLocked || isSpinning) return;
    triggerAnimation("prev", onAnterior);
  };

  return (
    <div className="[perspective:1200px] [transform-style:preserve-3d] flex flex-col items-center justify-center w-full max-w-[750px] mx-auto mt-1 mb-[18px] -translate-y-[25px] select-none">
      {/* MODO RULETA CINEMATOGRÁFICA 3D (GIRO ORBITAL DE 4 SEGUNDOS — ESCENARIO CENTRAL 625 PX) */}
      {isSpinning ? (
        <div className="w-full max-w-[625px] mx-auto flex flex-col items-center justify-center animate-fadeIn">
          <RouletteCarousel3D
            currentContent={currentContent}
            nextContent={nextContent}
            poolItems={poolItems}
            isSpinning={isSpinning}
            onSpinComplete={onSpinComplete}
          />

          {/* Controles en modo Ruleta (deshabilitados durante el giro, 100% aislados) */}
          <div className="flex items-center justify-center gap-14 mt-3 z-20">
            <div className="flex flex-col items-center">
              <span className="text-[11px] font-medium text-zinc-600 text-center tracking-wide">
                Anterior
              </span>
              <button
                disabled={true}
                className="w-8 h-8 rounded-full bg-neutral-950/40 border border-transparent flex items-center justify-center text-zinc-600 mx-auto mt-1 opacity-20 cursor-not-allowed"
                title="Anterior"
              >
                <ChevronLeft size={16} />
              </button>
            </div>

            <div className="flex flex-col items-center">
              <span className="text-[11px] font-semibold text-violet-400 text-center tracking-wide animate-pulse flex items-center gap-1.5">
                <Loader2 size={11} className="animate-spin text-violet-400" />
                Girando ruleta...
              </span>
              <button
                disabled={true}
                className="w-8 h-8 rounded-full bg-violet-950/40 border border-violet-500/30 flex items-center justify-center text-violet-300 mx-auto mt-1 opacity-40 cursor-not-allowed shadow-lg shadow-violet-950/50"
                title="Girando..."
              >
                <ChevronRight size={16} />
              </button>
            </div>
          </div>
        </div>
      ) : (
        /* MODO NORMAL ESTÁNDAR (3 SLOTS CINEMATOGRÁFICOS — CONTENEDOR 750 PX CON PERSPECTIVA 3D REAL) */
        <div className="w-full max-w-[750px] mx-auto flex items-center justify-center gap-1 sm:gap-2 md:gap-3 [perspective:900px] [transform-style:preserve-3d]">
          {/* Slot Izquierdo (Anterior - 255 × 382 px) */}
          <div className="flex flex-col items-center shrink-0 [perspective:900px] [transform-style:preserve-3d]">
            {/* Wrapper de Posicionamiento y Perspectiva Local */}
            <div
              role="button"
              tabIndex={hasPrevious ? 0 : -1}
              aria-label={hasPrevious ? "Ver recomendación anterior" : undefined}
              aria-disabled={!hasPrevious}
              onClick={hasPrevious ? handleAnterior : undefined}
              onKeyDown={(e) => {
                if (hasPrevious && (e.key === "Enter" || e.key === " ")) {
                  e.preventDefault();
                  handleAnterior();
                }
              }}
              className={`relative w-[255px] h-[382px] [perspective:900px] [transform-style:preserve-3d] ${
                hasPrevious
                  ? "opacity-100 cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-violet-500/70"
                  : "opacity-0 pointer-events-none"
              }`}
            >
              {hasPrevious ? (
                /* Wrapper 3D dedicado: Rotación, Profundidad y Escala */
                <div
                  className={`w-full h-full relative [transform-style:preserve-3d] transition-transform duration-400 ease-[cubic-bezier(0.22,1,0.36,1)] rounded-2xl ${
                    navDirection === "next"
                      ? "animate-carousel-next-left"
                      : "carousel-card-left"
                  }`}
                >
                  {leftCard}
                  {/* Sombra direccional de oclusión hacia la tarjeta central */}
                  <div className="absolute inset-0 rounded-2xl pointer-events-none bg-gradient-to-r from-transparent via-black/10 to-black/45" />
                </div>
              ) : (
                <div
                  className="w-[255px] h-[382px] invisible pointer-events-none shrink-0"
                  aria-hidden="true"
                />
              )}
            </div>

            {/* Controles inferiores de Anterior */}
            <div className="flex flex-col items-center mt-2.5">
              <span className="text-[11px] font-medium text-zinc-400 text-center tracking-wide">
                Anterior
              </span>
              <button
                onClick={hasPrevious ? handleAnterior : undefined}
                disabled={!hasPrevious || isLocked || isSpinning}
                aria-label="Ver recomendación anterior"
                className={`w-8 h-8 rounded-full bg-neutral-900/80 border border-white/10 flex items-center justify-center text-zinc-300 mx-auto mt-1 transition-all duration-300 ${
                  hasPrevious
                    ? "cursor-pointer hover:border-violet-500/50 hover:text-white hover:bg-neutral-800 hover:scale-105 active:scale-95 shadow-lg shadow-black/40 focus:outline-none focus-visible:ring-2 focus-visible:ring-violet-500"
                    : "opacity-25 cursor-not-allowed border-transparent bg-neutral-950/40"
                }`}
                title="Anterior"
              >
                <ChevronLeft size={16} />
              </button>
            </div>
          </div>

          {/* Slot Central (Activa - 335 × 458 px) */}
          <div className="flex flex-col items-center shrink-0 z-20 [perspective:900px] [transform-style:preserve-3d]">
            <div
              className={`w-[335px] h-[458px] [transform-style:preserve-3d] z-20 rounded-2xl border border-violet-500/40 transition-all duration-400 ease-[cubic-bezier(0.22,1,0.36,1)] motion-reduce:transition-none motion-reduce:transform-none ${
                navDirection === "next"
                  ? "animate-carousel-next-center"
                  : navDirection === "prev"
                  ? "animate-carousel-prev-center"
                  : "carousel-card-center"
              }`}
            >
              {centerCard}
            </div>
          </div>

          {/* Slot Derecho (Mystery Card - 255 × 382 px) */}
          <div className="flex flex-col items-center shrink-0 [perspective:900px] [transform-style:preserve-3d]">
            {/* Wrapper de Posicionamiento y Perspectiva Local */}
            <div
              role="button"
              tabIndex={0}
              aria-label="Obtener siguiente recomendación"
              onClick={handleSiguiente}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  handleSiguiente();
                }
              }}
              className="relative w-[255px] h-[382px] [perspective:900px] [transform-style:preserve-3d] opacity-100 cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-violet-500/70"
            >
              {/* Wrapper 3D dedicado: Rotación, Profundidad y Escala */}
              <div
                className={`w-full h-full relative [transform-style:preserve-3d] transition-transform duration-400 ease-[cubic-bezier(0.22,1,0.36,1)] rounded-2xl ${
                  navDirection === "prev"
                    ? "animate-carousel-prev-right"
                    : "carousel-card-right"
                }`}
              >
                {rightCard}
                {/* Sombra direccional de oclusión hacia la tarjeta central */}
                <div className="absolute inset-0 rounded-2xl pointer-events-none bg-gradient-to-l from-transparent via-black/10 to-black/45" />
              </div>
            </div>

            {/* Controles inferiores de Siguiente */}
            <div className="flex flex-col items-center mt-2.5">
              <span className="text-[11px] font-medium text-zinc-400 text-center tracking-wide">
                Siguiente
              </span>
              <button
                onClick={handleSiguiente}
                disabled={isLocked || isSpinning}
                aria-label="Obtener siguiente recomendación"
                className="w-8 h-8 rounded-full bg-neutral-900/80 border border-white/10 flex items-center justify-center text-zinc-300 mx-auto mt-1 hover:border-violet-500/50 hover:text-white hover:bg-neutral-800 hover:scale-105 active:scale-95 shadow-lg shadow-black/40 transition-all duration-300 cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-violet-500 disabled:opacity-50"
                title="Siguiente"
              >
                <ChevronRight size={16} />
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default CinemaCarousel;
