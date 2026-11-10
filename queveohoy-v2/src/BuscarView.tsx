import React, { useState, useEffect, useRef, useMemo } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import {
  Search,
  X,
  Loader2,
  Film,
  Tv,
  Play,
  Plus,
  Check,
  BookmarkCheck,
  CheckCircle2,
  Sparkles,
  Filter,
  RotateCcw,
  Tag,
  ChevronDown
} from "lucide-react";

export interface ResultadoBusqueda {
  tmdb_id: number | null;
  mal_id: number | null;
  tipo: string; // "MOVIE" | "TV"
  es_anime: boolean;
  titulo: string;
  titulo_original?: string | null;
  poster_url?: string | null;
  backdrop_url?: string | null;
  sinopsis?: string | null;
  fecha_estreno?: string | null;
  generos?: string | null;
  ya_en_biblioteca: boolean;
  estado_actual?: string | null;
}

interface ObraBusquedaPayload {
  tmdb_id: number | null;
  mal_id: number | null;
  tipo: string;
  es_anime: boolean;
  titulo: string;
  titulo_original?: string | null;
  poster_url?: string | null;
  backdrop_url?: string | null;
  sinopsis?: string | null;
  fecha_estreno?: string | null;
  generos?: string | null;
}

import {
  TipoContenido,
  GeneroConteo,
  TIPOS_CONFIG,
  GENRES_FALLBACK,
  normalizeGenre,
  parseGeneros,
  matchTipoSeleccionado,
  matchesAllGenres,
} from "./utils/filters";

export type { TipoContenido, GeneroConteo };
export { TIPOS_CONFIG, GENRES_FALLBACK, normalizeGenre, parseGeneros, matchTipoSeleccionado, matchesAllGenres };

interface BuscarViewProps {
  onShowToast?: (message: string) => void;
  onReanudar?: (item: any) => void;
}

/**
 * Resuelve la URL o ruta local del póster de forma infalible:
 * - Si es http/https, la devuelve directamente.
 * - Si empieza con '/' (path relativo de TMDB), antepone el CDN de TMDB w500.
 * - Si es una ruta absoluta en el disco local, la pasa por convertFileSrc.
 */
export function resolvePosterSrc(rawPoster?: string | null): string {
  if (!rawPoster) return "";
  const trimmed = rawPoster.trim();
  if (!trimmed) return "";
  if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
    return trimmed;
  }
  if (trimmed.startsWith("/")) {
    return `https://image.tmdb.org/t/p/w500${trimmed}`;
  }
  return convertFileSrc(trimmed);
}

export function BuscarView({ onShowToast, onReanudar }: BuscarViewProps) {
  const [query, setQuery] = useState("");
  const [selectedTipos, setSelectedTipos] = useState<TipoContenido[]>([]);
  const [selectedGeneros, setSelectedGeneros] = useState<string[]>([]);
  const [catalogo, setCatalogo] = useState<ResultadoBusqueda[]>([]);
  const [resultadosOnline, setResultadosOnline] = useState<ResultadoBusqueda[]>([]);
  const [availableGenres, setAvailableGenres] = useState<GeneroConteo[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadingCatalogo, setLoadingCatalogo] = useState(true);
  const [savingKey, setSavingKey] = useState<string | null>(null);

  // Popover de géneros
  const [isGenreOpen, setIsGenreOpen] = useState(false);
  const [genreSearch, setGenreSearch] = useState("");
  const popoverRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // 1. Cargar catálogo normal inicial y lista de géneros disponibles desde la base local
  useEffect(() => {
    let isMounted = true;
    setLoadingCatalogo(true);

    Promise.all([
      invoke<ResultadoBusqueda[]>("get_catalogo").catch((err) => {
        console.error("Error al cargar catálogo inicial:", err);
        return [] as ResultadoBusqueda[];
      }),
      invoke<GeneroConteo[]>("get_generos_disponibles_hoy").catch((err) => {
        console.error("Error al cargar géneros disponibles:", err);
        return [] as GeneroConteo[];
      })
    ]).then(([cat, genres]) => {
      if (!isMounted) return;
      setCatalogo(cat || []);
      if (genres && genres.length > 0) {
        setAvailableGenres(genres);
      } else {
        setAvailableGenres(GENRES_FALLBACK.map((g) => ({ nombre: g, count: 0 })));
      }
      setLoadingCatalogo(false);
    });

    return () => {
      isMounted = false;
    };
  }, []);

  // Cerrar popover de géneros al hacer clic fuera o pulsar Escape
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(event.target as Node)) {
        setIsGenreOpen(false);
      }
    };
    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setIsGenreOpen(false);
    };
    if (isGenreOpen) {
      document.addEventListener("mousedown", handleClickOutside);
      document.addEventListener("keydown", handleEscape);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [isGenreOpen]);

  // 2. Debounce para búsqueda online en TMDB/Jikan cuando query >= 2 caracteres
  useEffect(() => {
    const trimmed = query.trim();
    if (trimmed.length < 2) {
      setResultadosOnline([]);
      setLoading(false);
      return;
    }

    setLoading(true);
    const timer = setTimeout(async () => {
      try {
        const res = await invoke<ResultadoBusqueda[]>("buscar_online", {
          query: trimmed,
          tipoFiltro: null,
        });
        setResultadosOnline(res || []);
      } catch (err) {
        console.error("Error buscando online:", err);
        setResultadosOnline([]);
      } finally {
        setLoading(false);
      }
    }, 450);

    return () => clearTimeout(timer);
  }, [query]);

  // 3. Manejadores de selección de filtros
  const toggleTipo = (tipo: TipoContenido) => {
    setSelectedTipos((prev) => {
      const exists = prev.includes(tipo);
      const next = exists ? prev.filter((t) => t !== tipo) : [...prev, tipo];
      // Si el usuario seleccionó los tres, se normaliza a vacío (Todos)
      if (next.length === 3) {
        return [];
      }
      return next;
    });
  };

  const handleSelectTodosTipos = () => {
    setSelectedTipos([]);
  };

  const toggleGenero = (genero: string) => {
    setSelectedGeneros((prev) => {
      if (prev.includes(genero)) {
        return prev.filter((g) => g !== genero);
      } else {
        return [...prev, genero];
      }
    });
  };

  const handleClearFiltros = () => {
    setSelectedTipos([]);
    setSelectedGeneros([]);
  };

  const handleClearQuery = () => {
    setQuery("");
    setResultadosOnline([]);
    inputRef.current?.focus();
  };

  const getItemKey = (item: ResultadoBusqueda): string => {
    if (item.tmdb_id) return `tmdb_${item.tmdb_id}`;
    if (item.mal_id) return `mal_${item.mal_id}`;
    return `${item.titulo}_${item.fecha_estreno || ""}`;
  };

  // 4. Filtrado combinado en memoria de la fuente activa (Online o Catálogo normal)
  const isOnlineSearchActive = query.trim().length >= 2;
  const fuenteActiva = isOnlineSearchActive ? resultadosOnline : catalogo;

  const filteredResultados = useMemo(() => {
    return fuenteActiva.filter((item) => {
      // A. Filtro por tipo de contenido con la tabla de reglas definitiva
      if (!matchTipoSeleccionado(item, selectedTipos)) {
        return false;
      }

      // B. Filtro por género (acumulativo e independiente: la obra debe contener todos los géneros elegidos)
      if (!matchesAllGenres(item.generos, selectedGeneros)) {
        return false;
      }

      // C. Filtro adicional de texto si el usuario escribió un carácter sobre el catálogo
      if (!isOnlineSearchActive && query.trim().length === 1) {
        const q = query.trim().toLowerCase();
        const matchTitulo =
          item.titulo.toLowerCase().includes(q) ||
          (item.titulo_original && item.titulo_original.toLowerCase().includes(q));
        if (!matchTitulo) return false;
      }

      return true;
    });
  }, [fuenteActiva, selectedTipos, selectedGeneros, isOnlineSearchActive, query]);

  // Lista de géneros para el popover filtrados por el buscador interno
  const filteredGenresList = useMemo(() => {
    if (!genreSearch.trim()) return availableGenres;
    const q = normalizeGenre(genreSearch);
    return availableGenres.filter((g) => normalizeGenre(g.nombre).includes(q));
  }, [availableGenres, genreSearch]);

  const totalFiltrosActivos = selectedTipos.length + selectedGeneros.length;
  const isTodosActive = selectedTipos.length === 0;

  const handleSave = async (
    item: ResultadoBusqueda,
    nuevoEstado: "pendiente" | "en_progreso" | "terminada"
  ) => {
    const key = getItemKey(item);
    setSavingKey(key);

    try {
      const payload: ObraBusquedaPayload = {
        tmdb_id: item.tmdb_id,
        mal_id: item.mal_id,
        tipo: item.tipo,
        es_anime: item.es_anime,
        titulo: item.titulo,
        titulo_original: item.titulo_original ?? null,
        poster_url: item.poster_url ?? null,
        backdrop_url: item.backdrop_url ?? null,
        sinopsis: item.sinopsis ?? null,
        fecha_estreno: item.fecha_estreno ?? null,
        generos: item.generos ?? null,
      };

      const guardado = await invoke<any>("guardar_obra_busqueda", {
        obra: payload,
        estado: nuevoEstado,
      });

      // Actualizar estado local inmediato tanto en catálogo como en online
      const updater = (prev: ResultadoBusqueda[]) =>
        prev.map((r) =>
          getItemKey(r) === key
            ? { ...r, ya_en_biblioteca: true, estado_actual: nuevoEstado }
            : r
        );

      setResultadosOnline(updater);
      setCatalogo(updater);

      if (nuevoEstado === "en_progreso") {
        onShowToast?.(`"${item.titulo}" agregada a 'En progreso'`);
        if (onReanudar) {
          onReanudar(guardado);
        }
      } else if (nuevoEstado === "pendiente") {
        onShowToast?.(`"${item.titulo}" agregada a 'Para después'`);
      } else if (nuevoEstado === "terminada") {
        onShowToast?.(`"${item.titulo}" marcada como 'Ya la vi'`);
      }
    } catch (err) {
      console.error("Error al guardar obra:", err);
      onShowToast?.("Error al guardar la obra en la biblioteca");
    } finally {
      setSavingKey(null);
    }
  };

  return (
    <div className="w-full max-w-7xl mx-auto px-4 sm:px-6 py-6 flex flex-col gap-6">
      {/* 1. Header y Barra de Filtros */}
      <div className="flex flex-col gap-4">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
          <div>
            <h1 className="text-2xl font-bold text-white flex items-center gap-2.5">
              <Search className="text-violet-500" size={24} />
              Buscador y Catálogo
            </h1>
            <p className="text-xs sm:text-sm text-zinc-400 mt-0.5">
              Explora y combina filtros de tipo y género, o busca en vivo en TMDB y MyAnimeList
            </p>
          </div>

          {/* Controles de Filtros Principales (Tipos + Géneros + Limpiar) */}
          <div className="flex flex-wrap items-center gap-2.5 self-start lg:self-auto">
            {/* Selector Multi-tipo de Contenido */}
            <div className="flex items-center bg-zinc-900/90 rounded-xl p-1 border border-zinc-800/80 shadow-md">
              <button
                onClick={handleSelectTodosTipos}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer ${
                  isTodosActive
                    ? "bg-violet-600 text-white shadow-sm"
                    : "text-zinc-400 hover:text-zinc-200 hover:bg-white/5"
                }`}
                title="Mostrar todo el catálogo"
              >
                <span>Todos</span>
              </button>

              {TIPOS_CONFIG.map(({ id, label, icon: Icon }) => {
                const isSelected = selectedTipos.includes(id);
                return (
                  <button
                    key={id}
                    onClick={() => toggleTipo(id)}
                    className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer ${
                      isSelected
                        ? "bg-violet-600 text-white shadow-sm"
                        : "text-zinc-400 hover:text-zinc-200 hover:bg-white/5"
                    }`}
                    title={
                      isSelected
                        ? `Quitar filtro de ${label}`
                        : `Agregar filtro de ${label}`
                    }
                  >
                    <Icon size={13} />
                    <span>{label}</span>
                  </button>
                );
              })}
            </div>

            {/* Desplegable / Popover de Géneros */}
            <div className="relative" ref={popoverRef}>
              <button
                onClick={() => setIsGenreOpen(!isGenreOpen)}
                className={`flex items-center gap-2 px-3 py-2 rounded-xl text-xs font-medium transition-all cursor-pointer border shadow-sm ${
                  selectedGeneros.length > 0
                    ? "bg-violet-600/20 border-violet-500/50 text-violet-200"
                    : "bg-zinc-900/90 hover:bg-zinc-800/90 border-zinc-800/80 text-zinc-300"
                }`}
                title="Filtrar por género"
              >
                <Tag size={13} className={selectedGeneros.length > 0 ? "text-violet-400" : "text-zinc-400"} />
                <span>
                  {selectedGeneros.length === 0
                    ? "Géneros"
                    : `Géneros (${selectedGeneros.length})`}
                </span>
                <ChevronDown size={13} className="text-zinc-400" />
              </button>

              {/* Popover flotante de selección de géneros */}
              {isGenreOpen && (
                <div className="absolute right-0 top-full mt-2 w-80 p-3.5 bg-zinc-900/95 backdrop-blur-xl border border-zinc-700/80 rounded-2xl shadow-2xl shadow-black/90 z-50 flex flex-col gap-3 animate-in fade-in zoom-in-95 duration-150">
                  <div className="flex items-center justify-between border-b border-zinc-800 pb-2">
                    <span className="text-xs font-semibold text-zinc-200 flex items-center gap-1.5">
                      <Filter size={13} className="text-violet-400" />
                      Filtrar por género
                    </span>
                    {selectedGeneros.length > 0 && (
                      <button
                        onClick={() => setSelectedGeneros([])}
                        className="text-[11px] text-violet-400 hover:text-violet-300 font-medium cursor-pointer"
                      >
                        Desmarcar todos
                      </button>
                    )}
                  </div>

                  {/* Input de búsqueda interna en lista de géneros */}
                  <div className="relative">
                    <input
                      type="text"
                      value={genreSearch}
                      onChange={(e) => setGenreSearch(e.target.value)}
                      placeholder="Buscar género..."
                      className="w-full bg-zinc-950/80 border border-zinc-800 rounded-lg px-2.5 py-1.5 text-xs text-white placeholder-zinc-500 focus:outline-none focus:border-violet-500"
                    />
                    {genreSearch && (
                      <button
                        onClick={() => setGenreSearch("")}
                        className="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300"
                      >
                        <X size={12} />
                      </button>
                    )}
                  </div>

                  {/* Chips seleccionables de géneros */}
                  <div className="flex flex-wrap gap-1.5 max-h-56 overflow-y-auto pr-1">
                    {filteredGenresList.map((g) => {
                      const isSel = selectedGeneros.includes(g.nombre);
                      return (
                        <button
                          key={g.nombre}
                          onClick={() => toggleGenero(g.nombre)}
                          className={`text-[11px] px-2.5 py-1 rounded-lg border font-medium transition-all cursor-pointer flex items-center gap-1 ${
                            isSel
                              ? "bg-violet-600/30 text-violet-200 border-violet-500/60 shadow-sm"
                              : "bg-white/5 hover:bg-white/10 text-zinc-400 hover:text-zinc-200 border-white/5"
                          }`}
                        >
                          <span>{g.nombre}</span>
                          {g.count > 0 && (
                            <span className="text-[9px] text-zinc-500">
                              ({g.count})
                            </span>
                          )}
                        </button>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>

            {/* Botón Limpiar Filtros cuando hay filtros activos */}
            {totalFiltrosActivos > 0 && (
              <button
                onClick={handleClearFiltros}
                className="flex items-center gap-1.5 px-3 py-2 rounded-xl text-xs font-medium text-violet-300 hover:text-white bg-violet-500/10 hover:bg-violet-500/20 border border-violet-500/30 transition-all cursor-pointer shadow-sm"
                title="Limpiar todos los filtros"
              >
                <RotateCcw size={12} />
                <span>Limpiar filtros ({totalFiltrosActivos})</span>
              </button>
            )}
          </div>
        </div>

        {/* Input de Búsqueda con Iconos y Debounce */}
        <div className="relative w-full max-w-3xl">
          <div className="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none text-zinc-500">
            <Search size={18} />
          </div>
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Buscar por título (ej. Inception, Breaking Bad, Frieren)..."
            className="w-full bg-zinc-900/90 border border-zinc-800 text-white text-sm rounded-2xl pl-11 pr-11 py-3 focus:outline-none focus:border-violet-500 focus:ring-1 focus:ring-violet-500/50 transition-all placeholder-zinc-500 shadow-xl shadow-black/30"
          />
          <div className="absolute inset-y-0 right-0 pr-4 flex items-center">
            {loading ? (
              <Loader2 size={18} className="animate-spin text-violet-400" />
            ) : query.length > 0 ? (
              <button
                onClick={handleClearQuery}
                className="text-zinc-500 hover:text-zinc-300 p-1 rounded-full transition-colors cursor-pointer"
                title="Limpiar texto de búsqueda"
              >
                <X size={16} />
              </button>
            ) : null}
          </div>
        </div>

        {/* Fila de Filtros Activos con botones para desmarcarlos individualmente */}
        {totalFiltrosActivos > 0 && (
          <div className="flex items-center gap-2 flex-wrap text-xs pt-1">
            <span className="text-zinc-500 font-medium flex items-center gap-1">
              <Filter size={12} />
              Filtros activos:
            </span>

            {/* Tipos activos */}
            {selectedTipos.map((tipo) => {
              const cfg = TIPOS_CONFIG.find((t) => t.id === tipo);
              const Icon = cfg?.icon || Sparkles;
              return (
                <span
                  key={tipo}
                  className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-violet-600/20 text-violet-300 border border-violet-500/40 text-[11px] font-medium"
                >
                  <Icon size={11} />
                  <span>{cfg?.label || tipo}</span>
                  <button
                    onClick={() => toggleTipo(tipo)}
                    className="hover:text-white cursor-pointer ml-0.5"
                    title={`Remover filtro ${cfg?.label}`}
                  >
                    <X size={11} />
                  </button>
                </span>
              );
            })}

            {/* Géneros activos */}
            {selectedGeneros.map((gen) => (
              <span
                key={gen}
                className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-indigo-600/20 text-indigo-300 border border-indigo-500/40 text-[11px] font-medium"
              >
                <Tag size={11} />
                <span>{gen}</span>
                <button
                  onClick={() => toggleGenero(gen)}
                  className="hover:text-white cursor-pointer ml-0.5"
                  title={`Remover filtro ${gen}`}
                >
                  <X size={11} />
                </button>
              </span>
            ))}
          </div>
        )}
      </div>

      {/* 2. Resultados / Catálogo / Estados */}
      {loadingCatalogo && catalogo.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-24 text-center gap-3">
          <Loader2 size={36} className="animate-spin text-violet-500" />
          <p className="text-sm text-zinc-400">Cargando catálogo...</p>
        </div>
      ) : loading && resultadosOnline.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-24 text-center gap-3">
          <Loader2 size={36} className="animate-spin text-violet-500" />
          <p className="text-sm text-zinc-400">
            Consultando TMDB y MyAnimeList...
          </p>
        </div>
      ) : filteredResultados.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-center gap-3">
          <div className="p-4 rounded-full bg-zinc-900 border border-zinc-800 text-zinc-500">
            <Film size={36} />
          </div>
          <h2 className="text-base font-semibold text-zinc-300">
            {isOnlineSearchActive
              ? `No se encontraron resultados para "${query}" con los filtros seleccionados`
              : "No se encontraron obras para los filtros seleccionados"}
          </h2>
          <p className="text-xs sm:text-sm text-zinc-500 max-w-md">
            {totalFiltrosActivos > 0
              ? "Prueba a desmarcar algunos filtros de tipo o género para ampliar los resultados."
              : "Prueba con otro término de búsqueda o verifica la ortografía."}
          </p>
          {totalFiltrosActivos > 0 && (
            <button
              onClick={handleClearFiltros}
              className="mt-2 flex items-center gap-1.5 px-4 py-2 rounded-xl text-xs font-medium text-white bg-violet-600 hover:bg-violet-500 shadow-md transition-all cursor-pointer"
            >
              <RotateCcw size={13} />
              Limpiar filtros
            </button>
          )}
        </div>
      ) : (
        <div className="flex flex-col gap-3 pb-20">
          {/* Contador de obras mostradas */}
          <div className="flex items-center justify-between text-xs text-zinc-500 px-1">
            <span>
              {isOnlineSearchActive
                ? `Mostrando ${filteredResultados.length} resultados online para "${query}"`
                : `Mostrando ${filteredResultados.length} obras del catálogo`}
            </span>
          </div>

          {/* Rejilla de Obras */}
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {filteredResultados.map((item) => {
              const key = getItemKey(item);
              const isSaving = savingKey === key;
              return (
                <ResultadoCard
                  key={key}
                  item={item}
                  isSaving={isSaving}
                  onSave={handleSave}
                  onReanudar={onReanudar}
                />
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

interface ResultadoCardProps {
  item: ResultadoBusqueda;
  isSaving: boolean;
  onSave: (
    item: ResultadoBusqueda,
    estado: "pendiente" | "en_progreso" | "terminada"
  ) => void;
  onReanudar?: (item: any) => void;
}

function ResultadoCard({
  item,
  isSaving,
  onSave,
  onReanudar,
}: ResultadoCardProps) {
  const [imgError, setImgError] = useState(false);

  const year = item.fecha_estreno ? item.fecha_estreno.split("-")[0] : null;

  const generosList: string[] = React.useMemo(() => {
    return parseGeneros(item.generos).slice(0, 3);
  }, [item.generos]);

  // Resolución robusta del póster tanto para URLs absolutas de TMDB/Jikan como relativas de SQLite o locales
  const posterSrc = React.useMemo(() => {
    return resolvePosterSrc(item.poster_url);
  }, [item.poster_url]);

  const renderBadge = () => {
    if (item.es_anime) {
      return (
        <span className="text-[10px] font-semibold px-1.5 py-0.5 rounded bg-pink-500/20 text-pink-300 border border-pink-500/30">
          ANIME
        </span>
      );
    }
    if (item.tipo === "MOVIE") {
      return (
        <span className="text-[10px] font-semibold px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-300 border border-amber-500/30">
          PELÍCULA
        </span>
      );
    }
    return (
      <span className="text-[10px] font-semibold px-1.5 py-0.5 rounded bg-blue-500/20 text-blue-300 border border-blue-500/30">
        SERIE
      </span>
    );
  };

  const renderLibraryBadge = () => {
    const estado = item.estado_actual;
    if (estado === "en_progreso") {
      return (
        <div className="w-full flex items-center justify-between gap-2 py-1.5 px-3 rounded-xl bg-blue-500/10 border border-blue-500/20 text-blue-300 text-xs font-medium">
          <span className="flex items-center gap-1.5">
            <Play size={12} className="text-blue-400 fill-blue-400" />
            En progreso
          </span>
          <button
            onClick={() => onReanudar?.(item)}
            className="text-[11px] text-blue-400 hover:text-blue-200 hover:underline cursor-pointer font-medium"
          >
            Abrir en Hoy →
          </button>
        </div>
      );
    }
    if (estado === "pendiente") {
      return (
        <div className="w-full flex items-center justify-between gap-2 py-1.5 px-3 rounded-xl bg-purple-500/10 border border-purple-500/20 text-purple-300 text-xs font-medium">
          <span className="flex items-center gap-1.5">
            <BookmarkCheck size={13} className="text-purple-400" />
            Para después
          </span>
          <span className="text-[11px] text-purple-400/80">En lista</span>
        </div>
      );
    }
    if (estado === "terminada") {
      return (
        <div className="w-full flex items-center justify-between gap-2 py-1.5 px-3 rounded-xl bg-emerald-500/10 border border-emerald-500/20 text-emerald-300 text-xs font-medium">
          <span className="flex items-center gap-1.5">
            <CheckCircle2 size={13} className="text-emerald-400" />
            Ya la vi
          </span>
          <span className="text-[11px] text-emerald-400/80">Terminada</span>
        </div>
      );
    }
    return (
      <div className="w-full py-1.5 px-3 rounded-xl bg-violet-500/10 border border-violet-500/20 text-violet-300 text-xs font-medium text-center">
        En tu biblioteca
      </div>
    );
  };

  return (
    <div className="bg-zinc-900/80 hover:bg-zinc-900 border border-zinc-800/80 hover:border-zinc-700/80 rounded-2xl p-3 flex flex-col justify-between transition-all duration-200 hover:shadow-xl hover:shadow-black/50 group">
      {/* Contenido principal superior */}
      <div className="flex gap-3 items-start">
        {/* Póster */}
        <div className="relative aspect-[2/3] w-20 sm:w-24 shrink-0 rounded-xl overflow-hidden bg-zinc-800 border border-white/5 flex items-center justify-center">
          {posterSrc && !imgError ? (
            <img
              src={posterSrc}
              alt={item.titulo}
              className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
              onError={() => setImgError(true)}
              loading="lazy"
            />
          ) : (
            <div className="flex flex-col items-center justify-center text-zinc-600 gap-1 p-2 text-center">
              {item.tipo === "MOVIE" ? <Film size={22} /> : <Tv size={22} />}
              <span className="text-[9px] uppercase tracking-wider">Sin póster</span>
            </div>
          )}
        </div>

        {/* Metadatos y Sinopsis */}
        <div className="flex-1 min-w-0 flex flex-col gap-1">
          <div className="flex items-center gap-1.5 flex-wrap">
            {renderBadge()}
            {year && <span className="text-[11px] text-zinc-400 font-medium">{year}</span>}
          </div>

          <h3
            className="font-bold text-sm text-zinc-100 group-hover:text-white transition-colors line-clamp-1 leading-snug"
            title={item.titulo}
          >
            {item.titulo}
          </h3>

          {item.titulo_original && item.titulo_original !== item.titulo && (
            <p
              className="text-[11px] text-zinc-500 italic line-clamp-1"
              title={item.titulo_original}
            >
              {item.titulo_original}
            </p>
          )}

          {generosList.length > 0 && (
            <div className="flex flex-wrap gap-1 mt-0.5">
              {generosList.map((g) => (
                <span
                  key={g}
                  className="text-[10px] px-1.5 py-0.5 rounded-md bg-white/5 text-zinc-400 border border-white/5"
                >
                  {g}
                </span>
              ))}
            </div>
          )}

          {item.sinopsis && (
            <p className="text-xs text-zinc-400/90 line-clamp-2 mt-1 leading-relaxed">
              {item.sinopsis}
            </p>
          )}
        </div>
      </div>

      {/* Acciones o Estado en Biblioteca */}
      <div className="mt-3 pt-2.5 border-t border-zinc-800/60 flex items-center justify-between gap-1.5">
        {isSaving ? (
          <div className="w-full flex items-center justify-center gap-2 py-1.5 text-xs text-violet-400 font-medium">
            <Loader2 size={14} className="animate-spin" />
            Guardando en tu biblioteca...
          </div>
        ) : item.ya_en_biblioteca ? (
          renderLibraryBadge()
        ) : (
          <div className="w-full flex items-center gap-1.5">
            <button
              onClick={() => onSave(item, "pendiente")}
              className="flex-1 flex items-center justify-center gap-1 px-2 py-1.5 rounded-xl bg-white/5 hover:bg-white/10 text-zinc-300 hover:text-white border border-white/5 text-xs font-medium transition-colors cursor-pointer"
              title="Guardar en lista 'Para después'"
            >
              <Plus size={13} />
              <span>Para después</span>
            </button>

            <button
              onClick={() => onSave(item, "en_progreso")}
              className="flex-1 flex items-center justify-center gap-1 px-2 py-1.5 rounded-xl bg-violet-600 hover:bg-violet-500 text-white text-xs font-medium shadow-md transition-colors cursor-pointer"
              title="Empezar a ver ahora y abrir en Hoy"
            >
              <Play size={12} className="fill-white" />
              <span>Empezar</span>
            </button>

            <button
              onClick={() => onSave(item, "terminada")}
              className="flex items-center justify-center p-2 rounded-xl bg-white/5 hover:bg-emerald-600/20 hover:text-emerald-300 text-zinc-400 border border-white/5 transition-colors cursor-pointer"
              title="Marcar como 'Ya la vi'"
            >
              <Check size={14} />
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
