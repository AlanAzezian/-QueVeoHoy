import React from "react";
import { ArrowRight, Sparkles } from "lucide-react";

interface MysteryCardProps {
  onClick?: () => void;
  className?: string;
}

export const MysteryCard: React.FC<MysteryCardProps> = ({
  onClick,
  className = ""
}) => {
  return (
    <div
      onClick={onClick}
      className={`w-[255px] h-[382px] shrink-0 rounded-2xl overflow-hidden relative flex flex-col items-center justify-center border-2 border-dashed border-purple-500/45 group select-none shadow-[0_15px_35px_rgba(0,0,0,0.85)] hover:border-purple-400/80 transition-all duration-300 cursor-pointer ${className}`}
      style={{
        background: `
          radial-gradient(circle at 50% 45%, rgba(147, 51, 234, 0.35) 0%, transparent 60%),
          radial-gradient(circle at 30% 70%, rgba(67, 56, 202, 0.25) 0%, transparent 50%),
          linear-gradient(180deg, #110c1f 0%, #0a0714 100%)
        `
      }}
    >
      {/* 1. Bisel superior de luz translúcida */}
      <div className="absolute top-0 inset-x-0 h-[1px] bg-gradient-to-r from-transparent via-purple-300/30 to-transparent pointer-events-none" />

      {/* 2. Textura sutil de polvo estelar cósmico */}
      <div className="absolute inset-0 opacity-20 bg-[radial-gradient(white_1px,transparent_1px)] [background-size:20px_20px] pointer-events-none" />

      {/* 3. Halo cósmico difuso detrás del núcleo */}
      <div className="absolute w-44 h-44 rounded-full bg-purple-600/25 blur-2xl pointer-events-none animate-pulse" />

      {/* 4. Círculo central holográfico con efecto de vidrio */}
      <div
        className="relative z-10 w-28 h-28 rounded-full flex items-center justify-center border border-purple-400/40 bg-purple-950/40 backdrop-blur-md shadow-2xl select-none"
        style={{
          boxShadow: "0 0 50px rgba(147, 51, 234, 0.45), inset 0 0 20px rgba(168, 85, 247, 0.3)"
        }}
      >
        {/* Signo de interrogación protagonista */}
        <span
          className="font-extrabold text-6xl text-[#f5f3ff] select-none tracking-tight"
          style={{
            textShadow: "0 0 12px rgba(192, 132, 252, 0.9), 0 0 30px rgba(147, 51, 234, 0.7)"
          }}
        >
          ?
        </span>
      </div>

      {/* 5. Etiqueta informativa inferior visible y animada en hover */}
      <div className="absolute bottom-4 inset-x-0 flex items-center justify-center gap-1.5 text-xs font-semibold text-purple-200/70 group-hover:text-purple-100 opacity-60 group-hover:opacity-100 transition-all duration-300 pointer-events-none">
        <Sparkles size={12} className="text-purple-400 group-hover:rotate-12 transition-transform duration-300" />
        <span>Siguiente recomendación</span>
        <ArrowRight size={13} className="text-purple-400 transform group-hover:translate-x-1 transition-transform duration-300" />
      </div>
    </div>
  );
};

export default MysteryCard;
