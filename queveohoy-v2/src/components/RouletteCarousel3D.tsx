import React, { useEffect, useState, useRef } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Sparkles, Film, Tv } from "lucide-react";

export interface RouletteItem {
  id: number | string;
  titulo: string;
  titulo_original?: string;
  poster_url?: string | null;
  tipo?: string;
  fecha_estreno?: string;
  es_anime?: boolean | number;
}

interface RouletteCarousel3DProps {
  currentContent: RouletteItem | null;
  nextContent: RouletteItem | null;
  poolItems: RouletteItem[];
  isSpinning: boolean;
  onSpinComplete: () => void;
}

const TOTAL_SLOTS = 12;
const WINNING_INDEX = 7; // El slot 7 contendrá la nueva recomendación elegida
const FULL_TURNS = 4; // 4 vueltas completas de 360° = 1440°
const DEG_PER_SLOT = 360 / TOTAL_SLOTS; // 30° por tarjeta
const RADIUS = 365; // Radio calibrado para un ancho total visible de ~600-625 px en el escenario

const resolvePoster = (url?: string | null): string => {
  if (!url || typeof url !== "string" || !url.trim()) return "";
  const clean = url.trim();
  if (clean.startsWith("http://") || clean.startsWith("https://") || clean.startsWith("asset://") || clean.startsWith("blob:")) {
    return clean;
  }
  return convertFileSrc(clean);
};

export const RouletteCarousel3D: React.FC<RouletteCarousel3DProps> = ({
  currentContent,
  nextContent,
  poolItems,
  isSpinning,
  onSpinComplete,
}) => {
  const [rotationDeg, setRotationDeg] = useState(0);
  const [itemsOnRing, setItemsOnRing] = useState<RouletteItem[]>([]);
  const hasTriggeredCompleteRef = useRef(false);

  // Armar el anillo de 12 tarjetas reales combinando current, next y el catálogo real
  useEffect(() => {
    const ring: RouletteItem[] = [];
    const availablePool = poolItems.filter(
      p => p.id !== currentContent?.id && p.id !== nextContent?.id
    );

    for (let i = 0; i < TOTAL_SLOTS; i++) {
      if (i === 0 && currentContent) {
        // Slot 0: La recomendación actual (comienza de frente al usuario)
        ring.push(currentContent);
      } else if (i === WINNING_INDEX && nextContent) {
        // Slot WINNING_INDEX: La nueva recomendación que frenará de frente
        ring.push(nextContent);
      } else {
        // Slots intermedios: Películas, series y animes reales del catálogo
        const poolIndex = i % (availablePool.length || 1);
        const item = availablePool[poolIndex] || currentContent || {
          id: `slot-${i}`,
          titulo: "Película del Catálogo",
          tipo: "MOVIE"
        };
        ring.push(item);
      }
    }

    setItemsOnRing(ring);
  }, [currentContent, nextContent, poolItems]);

  // Manejar el giro de 4 segundos
  useEffect(() => {
    if (!isSpinning) {
      setRotationDeg(0);
      hasTriggeredCompleteRef.current = false;
      return;
    }

    // Comprobar accesibilidad prefers-reduced-motion
    const prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (prefersReducedMotion) {
      const timer = setTimeout(() => {
        onSpinComplete();
      }, 100);
      return () => clearTimeout(timer);
    }

    // Calcular ángulo objetivo exacto para que el slot ganador aterrice en 0° frontal
    // delta = -(4 vueltas * 360° + WINNING_INDEX * 30°) = -1650°
    const target = -(FULL_TURNS * 360 + WINNING_INDEX * DEG_PER_SLOT);

    // Iniciar rotación en el siguiente frame para que CSS transition la anime desde 0°
    const frame = requestAnimationFrame(() => {
      setRotationDeg(target);
    });

    const completionTimer = setTimeout(() => {
      if (!hasTriggeredCompleteRef.current) {
        hasTriggeredCompleteRef.current = true;
        onSpinComplete();
      }
    }, 4000); // 4.0 segundos exactos

    return () => {
      cancelAnimationFrame(frame);
      clearTimeout(completionTimer);
    };
  }, [isSpinning, onSpinComplete]);

  return (
    <div className="relative w-full max-w-[625px] h-[430px] mx-auto flex items-center justify-center [perspective:1200px] select-none overflow-visible">
      {/* Halo ambiental central de la ruleta */}
      <div className="absolute inset-0 m-auto w-72 h-72 rounded-full bg-violet-600/20 blur-3xl pointer-events-none -z-10 animate-pulse" />

      {/* Indicador de ruleta activa */}
      <div className="absolute top-0 left-1/2 -translate-x-1/2 z-30 flex items-center gap-1.5 px-3 py-1 rounded-full bg-violet-950/80 border border-violet-500/40 text-violet-300 text-xs font-semibold backdrop-blur-md shadow-lg shadow-violet-950/50">
        <Sparkles size={13} className="text-violet-400 animate-spin" />
        <span className="tracking-wide uppercase text-[11px]">Sorteando recomendación...</span>
      </div>

      {/* Contenedor cilíndrico rotatorio 3D */}
      <div
        className="relative w-[190px] h-[285px] [transform-style:preserve-3d] flex items-center justify-center"
        style={{
          transform: `rotateY(${rotationDeg}deg)`,
          transition: isSpinning
            ? "transform 4000ms cubic-bezier(0.12, 0.82, 0.18, 1)"
            : "none"
        }}
      >
        {itemsOnRing.map((item, index) => {
          const slotAngle = index * DEG_PER_SLOT;
          const poster = resolvePoster(item.poster_url);
          const isWinner = index === WINNING_INDEX;

          return (
            <div
              key={`${item.id}-${index}`}
              className="absolute inset-0 w-[190px] h-[285px] rounded-xl overflow-hidden [backface-visibility:hidden] [transform-style:preserve-3d] border border-white/10 shadow-[0_12px_28px_rgba(0,0,0,0.8)] bg-neutral-900 group"
              style={{
                transform: `rotateY(${slotAngle}deg) translateZ(${RADIUS}px)`,
              }}
            >
              {poster ? (
                <img
                  src={poster}
                  alt={item.titulo}
                  className="w-full h-full object-cover"
                  loading="eager"
                />
              ) : (
                <div className="w-full h-full flex flex-col items-center justify-center bg-neutral-900 p-3 text-center">
                  {item.tipo === "MOVIE" ? (
                    <Film size={28} className="text-violet-400 mb-1.5" />
                  ) : (
                    <Tv size={28} className="text-violet-400 mb-1.5" />
                  )}
                  <span className="text-[11px] font-semibold text-zinc-300 line-clamp-2">
                    {item.titulo}
                  </span>
                </div>
              )}

              {/* Overlay de gradiente */}
              <div className="absolute inset-0 bg-gradient-to-t from-neutral-950 via-neutral-950/40 to-transparent pointer-events-none" />

              {/* Título en la base */}
              <div className="absolute bottom-0 inset-x-0 p-2.5 flex flex-col items-center text-center">
                <span className="font-bold text-white text-[11px] tracking-wide uppercase line-clamp-1 drop-shadow">
                  {item.titulo}
                </span>
                {item.fecha_estreno && (
                  <span className="text-[9px] text-zinc-400 mt-0.5">
                    {item.fecha_estreno.slice(0, 4)}
                  </span>
                )}
              </div>

              {/* Indicador luminoso en la tarjeta ganadora cuando se aproxima */}
              {isWinner && (
                <div className="absolute inset-0 rounded-xl border-2 border-violet-500/80 pointer-events-none shadow-[0_0_25px_rgba(139,92,246,0.6)]" />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default RouletteCarousel3D;
