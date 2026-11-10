import React from "react";
import mysteryCardImg from "../assets/mystery-card.jpg";

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
      className={`w-[255px] h-[382px] shrink-0 rounded-2xl overflow-hidden relative flex flex-col items-center justify-center border border-white/10 group select-none shadow-[0_15px_35px_rgba(0,0,0,0.85)] cursor-pointer ${className}`}
    >
      {/* 1. Imagen visual principal de la Mystery Card (sin zoom ni deformación) */}
      <img
        src={mysteryCardImg}
        alt="Mystery Card"
        className="absolute inset-0 w-full h-full object-cover object-center select-none pointer-events-none"
        draggable={false}
      />

      {/* 2. Capa de gradiente cinematográfico muy sutil para integrarse con la atmósfera de la app */}
      <div className="absolute inset-0 bg-gradient-to-t from-black/40 via-transparent to-black/20 pointer-events-none" />

      {/* 3. Bisel superior sutil de luz translúcida */}
      <div className="absolute top-0 inset-x-0 h-[1px] bg-gradient-to-r from-transparent via-white/15 to-transparent pointer-events-none" />
    </div>
  );
};

export default MysteryCard;
