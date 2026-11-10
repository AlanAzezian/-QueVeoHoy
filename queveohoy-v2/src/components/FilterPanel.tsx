import React, { useState, useEffect, useRef } from "react";
import { Filter, X, Check, RotateCcw } from "lucide-react";
import {
  TipoContenido,
  GeneroConteo,
  TIPOS_CONFIG,
  GENRES_FALLBACK,
  FiltrosConfig
} from "../utils/filters";

interface FilterPanelProps {
  isOpen: boolean;
  onClose: () => void;
  appliedFiltros: FiltrosConfig;
  onApply: (filtros: FiltrosConfig) => void;
  onClear: () => void;
  availableGenres: GeneroConteo[];
}

export const FilterPanel: React.FC<FilterPanelProps> = ({
  isOpen,
  onClose,
  appliedFiltros,
  onApply,
  onClear,
  availableGenres
}) => {
  // Estado provisional independiente dentro del panel
  const [provisionalTipos, setProvisionalTipos] = useState<TipoContenido[]>(appliedFiltros.tipos);
  const [provisionalGeneros, setProvisionalGeneros] = useState<string[]>(appliedFiltros.generos);
  const [genreSearch, setGenreSearch] = useState("");

  const panelRef = useRef<HTMLDivElement>(null);

  // Sincronizar selección provisional al abrir el panel con los filtros aplicados actuales
  useEffect(() => {
    if (isOpen) {
      setProvisionalTipos(appliedFiltros.tipos);
      setProvisionalGeneros(appliedFiltros.generos);
      setGenreSearch("");
    }
  }, [isOpen, appliedFiltros]);

  // Cerrar al presionar Escape o al hacer clic fuera del panel
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    const handleClickOutside = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    const timer = setTimeout(() => {
      window.addEventListener("mousedown", handleClickOutside);
    }, 10);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("mousedown", handleClickOutside);
      clearTimeout(timer);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const toggleTipo = (tipo: TipoContenido) => {
    setProvisionalTipos((prev) =>
      prev.includes(tipo) ? prev.filter((t) => t !== tipo) : [...prev, tipo]
    );
  };

  const toggleGenero = (genero: string) => {
    setProvisionalGeneros((prev) =>
      prev.includes(genero) ? prev.filter((g) => g !== genero) : [...prev, genero]
    );
  };

  const handleApply = () => {
    onApply({
      tipos: provisionalTipos,
      generos: provisionalGeneros
    });
    onClose();
  };

  const handleClear = () => {
    setProvisionalTipos([]);
    setProvisionalGeneros([]);
    onClear();
    onClose();
  };

  // Lista de géneros disponibles
  const genresList = availableGenres && availableGenres.length > 0
    ? availableGenres.map((g) => g.nombre)
    : GENRES_FALLBACK;

  const filteredGenres = genreSearch.trim()
    ? genresList.filter((g) => g.toLowerCase().includes(genreSearch.toLowerCase().trim()))
    : genresList;

  const provisionalCount = provisionalTipos.length + provisionalGeneros.length;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-end p-4 sm:p-6 md:p-8 pointer-events-none">
      {/* Telón de fondo translúcido sutil para click-outside */}
      <div className="absolute inset-0 bg-black/40 backdrop-blur-[2px] pointer-events-auto" onClick={onClose} />

      {/* Contenedor del panel flotante */}
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label="Filtros de recomendaciones"
        className="relative z-10 w-full max-w-[380px] sm:max-w-[420px] bg-[#0c0916]/95 border border-violet-500/40 rounded-2xl shadow-[0_12px_40px_rgba(0,0,0,0.85),_0_0_25px_rgba(139,92,246,0.2)] backdrop-blur-xl p-5 flex flex-col gap-4 text-white pointer-events-auto animate-in fade-in zoom-in-95 duration-200 mt-10 max-h-[calc(100vh-6rem)] overflow-y-auto select-none"
      >
        {/* Cabecera del panel */}
        <div className="flex items-center justify-between pb-3 border-b border-white/10">
          <div className="flex items-center gap-2">
            <div className="w-7 h-7 rounded-lg bg-violet-600/30 border border-violet-400/40 flex items-center justify-center text-violet-300">
              <Filter size={14} />
            </div>
            <div>
              <h3 className="text-sm font-bold tracking-tight text-white flex items-center gap-2">
                Filtros de Recomendación
                {provisionalCount > 0 && (
                  <span className="text-[11px] font-semibold bg-violet-600/40 text-violet-200 border border-violet-400/30 px-1.5 py-0.2 rounded-full">
                    {provisionalCount}
                  </span>
                )}
              </h3>
              <p className="text-[11px] text-zinc-400">Personaliza lo que ves hoy</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-zinc-400 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
            title="Cerrar panel"
          >
            <X size={16} />
          </button>
        </div>

        {/* 1. SECCIÓN: TIPO DE CONTENIDO */}
        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-zinc-300 uppercase tracking-wider">
              Tipo de Contenido
            </span>
            {provisionalTipos.length > 0 && (
              <button
                onClick={() => setProvisionalTipos([])}
                className="text-[11px] text-violet-400 hover:text-violet-300 transition-colors cursor-pointer"
              >
                Todos los tipos
              </button>
            )}
          </div>
          <div className="grid grid-cols-3 gap-2">
            {TIPOS_CONFIG.map(({ id, label, icon: Icon }) => {
              const isSelected = provisionalTipos.includes(id);
              return (
                <button
                  key={id}
                  onClick={() => toggleTipo(id)}
                  className={`flex flex-col items-center justify-center gap-1.5 py-2.5 px-2 rounded-xl border text-xs font-medium transition-all duration-200 cursor-pointer ${
                    isSelected
                      ? "bg-violet-600/35 border-violet-400/70 text-white shadow-md shadow-violet-950/50 scale-[1.02]"
                      : "bg-neutral-900/60 border-white/10 text-zinc-300 hover:border-white/25 hover:text-white hover:bg-neutral-800/60"
                  }`}
                >
                  <Icon size={16} className={isSelected ? "text-violet-300" : "text-zinc-400"} />
                  <span className="leading-none">{label}</span>
                </button>
              );
            })}
          </div>
          <span className="text-[10.5px] text-zinc-400/80 italic">
            {provisionalTipos.length === 0
              ? "Sin selección: se recomiendan películas, series y anime."
              : provisionalTipos.length === 3
              ? "Todos los tipos seleccionados."
              : `Filtrando por: ${provisionalTipos.map(t => t === "peliculas" ? "Películas" : t === "series" ? "Series" : "Anime").join(", ")}`}
          </span>
        </div>

        {/* 2. SECCIÓN: GÉNEROS */}
        <div className="flex flex-col gap-2 pt-1 border-t border-white/10">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-zinc-300 uppercase tracking-wider">
              Géneros (Combinación AND)
            </span>
            {provisionalGeneros.length > 0 && (
              <button
                onClick={() => setProvisionalGeneros([])}
                className="text-[11px] text-violet-400 hover:text-violet-300 transition-colors cursor-pointer"
              >
                Limpiar géneros
              </button>
            )}
          </div>

          {/* Buscador rápido de géneros si la lista es extensa */}
          {genresList.length > 12 && (
            <input
              type="text"
              value={genreSearch}
              onChange={(e) => setGenreSearch(e.target.value)}
              placeholder="Buscar género..."
              className="w-full bg-neutral-950/70 border border-white/10 rounded-lg px-2.5 py-1 text-xs text-white placeholder-zinc-500 focus:outline-none focus:border-violet-500/50"
            />
          )}

          {/* Chips de géneros */}
          <div className="flex flex-wrap gap-1.5 max-h-44 overflow-y-auto pr-1 scrollbar-thin">
            {filteredGenres.map((g) => {
              const isSelected = provisionalGeneros.includes(g);
              return (
                <button
                  key={g}
                  onClick={() => toggleGenero(g)}
                  className={`flex items-center gap-1 px-2.5 py-1 rounded-full text-xs transition-all duration-150 cursor-pointer border ${
                    isSelected
                      ? "bg-violet-600/35 border-violet-400/80 text-white font-medium shadow-sm shadow-violet-950/60"
                      : "bg-neutral-900/50 border-white/10 text-zinc-300 hover:border-white/20 hover:text-white"
                  }`}
                >
                  {isSelected && <Check size={11} className="text-violet-300 shrink-0" />}
                  <span>{g}</span>
                </button>
              );
            })}
          </div>
          {provisionalGeneros.length > 1 && (
            <span className="text-[10.5px] text-violet-300/80">
              * La recomendación debe contener <strong>todos</strong> los géneros seleccionados.
            </span>
          )}
        </div>

        {/* 3. BOTONERA DE ACCIONES INFERIORES */}
        <div className="flex items-center justify-between pt-3 border-t border-white/10 gap-3">
          <button
            onClick={handleClear}
            className="flex items-center gap-1.5 px-3 py-2 rounded-xl text-xs font-medium text-zinc-400 hover:text-white hover:bg-white/5 border border-transparent hover:border-white/10 transition-colors cursor-pointer"
            title="Restablecer todos los filtros"
          >
            <RotateCcw size={13} />
            <span>Limpiar filtros</span>
          </button>

          <button
            onClick={handleApply}
            className="flex-1 bg-gradient-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 text-white font-semibold text-xs py-2 px-4 rounded-xl shadow-lg shadow-violet-950/50 border border-violet-400/30 flex items-center justify-center gap-1.5 transition-all cursor-pointer active:scale-95"
          >
            <Check size={13} />
            <span>Aplicar filtros</span>
          </button>
        </div>
      </div>
    </div>
  );
};
