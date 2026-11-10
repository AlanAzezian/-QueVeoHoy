import React from "react";

interface PosterCardProps {
  posterUrl?: string;
  title?: string;
  subtitle?: string;
  isCenter?: boolean;
  badge?: React.ReactNode;
  bottomAction?: React.ReactNode;
  progress?: {
    temp: number;
    ep: number;
    percent: number;
  } | null;
  onClick?: () => void;
  className?: string;
}

export const PosterCard: React.FC<PosterCardProps> = ({
  posterUrl,
  title,
  subtitle,
  isCenter = false,
  badge,
  bottomAction,
  progress,
  onClick,
  className = ""
}) => {
  if (isCenter) {
    return (
      <div
        onClick={onClick}
        role={onClick ? "button" : undefined}
        tabIndex={onClick ? 0 : undefined}
        onKeyDown={
          onClick
            ? (e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onClick();
                }
              }
            : undefined
        }
        className={`w-[335px] h-[458px] shrink-0 rounded-2xl overflow-hidden relative z-10 bg-neutral-900 group select-none shadow-[0_25px_50px_rgba(0,0,0,0.9),0_0_35px_rgba(139,92,246,0.3)] ${className}`}
      >
        {/* Badge superior */}
        {badge && (
          <div className="absolute top-3 right-3 z-20 pointer-events-auto">
            {badge}
          </div>
        )}

        {/* Imagen del póster */}
        {posterUrl ? (
          <img
            src={posterUrl}
            alt={title || "Póster"}
            className="w-full h-full object-cover select-none"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center bg-neutral-900 text-zinc-600 text-sm">
            Sin imagen
          </div>
        )}

        {/* Overlay inferior para contraste */}
        <div className="absolute inset-0 bg-gradient-to-t from-neutral-950/70 via-transparent to-black/20 pointer-events-none" />

        {/* Acción inferior interactiva (con aislamiento de eventos para evitar clics duplicados) */}
        {bottomAction && (
          <div
            className="absolute bottom-4 inset-x-0 flex justify-center z-20 pointer-events-auto"
            onClick={(e) => e.stopPropagation()}
          >
            {bottomAction}
          </div>
        )}
      </div>
    );
  }

  // Tarjeta lateral (Anterior / Secundaria)
  return (
    <div
      onClick={onClick}
      className={`w-[255px] h-[382px] shrink-0 rounded-2xl overflow-hidden border border-white/10 relative bg-neutral-900 group select-none shadow-lg shadow-black/50 ${className}`}
    >
      {posterUrl ? (
        <>
          <img
            src={posterUrl}
            alt={title || "Póster"}
            className="w-full h-full object-cover select-none"
          />
          <div className="absolute inset-0 bg-gradient-to-t from-neutral-950 via-neutral-950/40 to-transparent" />
          <div className="absolute bottom-0 inset-x-0 p-3 flex flex-col items-center text-center">
            {title && (
              <span className="font-bold text-white text-xs md:text-sm tracking-wide uppercase line-clamp-1">
                {title}
              </span>
            )}
            {subtitle && (
              <span className="text-[11px] text-zinc-400 line-clamp-1 mt-0.5">
                {subtitle}
              </span>
            )}
            {progress && (
              <div className="w-full mt-2">
                <div className="w-full h-1 bg-white/10 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-violet-500 rounded-full"
                    style={{ width: `${progress.percent}%` }}
                  />
                </div>
                <div className="flex justify-between items-center text-[10px] text-zinc-400 mt-1">
                  <span>T{progress.temp} · Cap {progress.ep}</span>
                  <span>{progress.percent}%</span>
                </div>
              </div>
            )}
          </div>
        </>
      ) : (
        <div className="w-full h-full flex items-center justify-center bg-neutral-900/50 text-zinc-600 text-xs">
          Sin contenido
        </div>
      )}
    </div>
  );
};
