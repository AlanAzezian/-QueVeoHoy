import React, { useState, useEffect, useRef } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { Search, X, Loader2, Film, Tv, Play, Plus, Check, BookmarkCheck, CheckCircle2, Sparkles } from "lucide-react";

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

interface BuscarViewProps {
  onShowToast?: (message: string) => void;
  onReanudar?: (item: any) => void;
}

type TipoFiltro = "todos" | "peliculas" | "series" | "anime";

const FILTROS: { id: TipoFiltro; label: string; icon: React.ComponentType<{ size?: number; className?: string }> }[] = [
  { id: "todos", label: "Todos", icon: Sparkles },
  { id: "peliculas", label: "Películas", icon: Film },
  { id: "series", label: "Series", icon: Tv },
  { id: "anime", label: "Anime", icon: Sparkles },
];

export function BuscarView({ onShowToast, onReanudar }: BuscarViewProps) {
  const [query, setQuery] = useState("");
  const [filtro, setFiltro] = useState<TipoFiltro>("todos");
  const [resultados, setResultados] = useState<ResultadoBusqueda[]>([]);
  const [loading, setLoading] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const [savingKey, setSavingKey] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Debounce search effect
  useEffect(() => {
    const trimmed = query.trim();
    if (trimmed.length < 2) {
      setResultados([]);
      setLoading(false);
      setHasSearched(false);
      return;
    }

    setLoading(true);
    const timer = setTimeout(async () => {
      try {
        const res = await invoke<ResultadoBusqueda[]>("buscar_online", {
          query: trimmed,
          tipoFiltro: filtro === "todos" ? null : filtro,
        });
        setResultados(res || []);
        setHasSearched(true);
      } catch (err) {
        console.error("Error buscando online:", err);
        setResultados([]);
      } finally {
        setLoading(false);
      }
    }, 450);

    return () => clearTimeout(timer);
  }, [query, filtro]);

  const handleClear = () => {
    setQuery("");
    setResultados([]);
    setHasSearched(false);
    inputRef.current?.focus();
  };

  const getItemKey = (item: ResultadoBusqueda): string => {
    if (item.tmdb_id) return `tmdb_${item.tmdb_id}`;
    if (item.mal_id) return `mal_${item.mal_id}`;
    return `${item.titulo}_${item.fecha_estreno || ""}`;
  };

  const handleSave = async (item: ResultadoBusqueda, nuevoEstado: "pendiente" | "en_progreso" | "terminada") => {
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

      // Actualizar estado local inmediato de la tarjeta
      setResultados(prev =>
        prev.map(r =>
          getItemKey(r) === key
            ? { ...r, ya_en_biblioteca: true, estado_actual: nuevoEstado }
            : r
        )
      );

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
      {/* Header y Buscador */}
      <div className="flex flex-col gap-4">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <h1 className="text-2xl font-bold text-white flex items-center gap-2.5">
              <Search className="text-violet-500" size={24} />
              Buscador Global
            </h1>
            <p className="text-xs sm:text-sm text-zinc-400 mt-0.5">
              Explora películas, series y anime para incorporar directamente a tu catálogo personal
            </p>
          </div>

          {/* Filtros de Categoría */}
          <div className="flex items-center bg-zinc-900/90 rounded-xl p-1 border border-zinc-800/80 shadow-md self-start sm:self-auto">
            {FILTROS.map(({ id, label, icon: Icon }) => (
              <button
                key={id}
                onClick={() => setFiltro(id)}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs sm:text-sm font-medium transition-all cursor-pointer ${
                  filtro === id
                    ? "bg-violet-600 text-white shadow-sm"
                    : "text-zinc-400 hover:text-zinc-200 hover:bg-white/5"
                }`}
              >
                <Icon size={14} />
                <span>{label}</span>
              </button>
            ))}
          </div>
        </div>

        {/* Input con Iconos y Debounce */}
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
            autoFocus
          />
          <div className="absolute inset-y-0 right-0 pr-4 flex items-center">
            {loading ? (
              <Loader2 size={18} className="animate-spin text-violet-400" />
            ) : query.length > 0 ? (
              <button
                onClick={handleClear}
                className="text-zinc-500 hover:text-zinc-300 p-1 rounded-full transition-colors cursor-pointer"
                title="Limpiar búsqueda"
              >
                <X size={16} />
              </button>
            ) : null}
          </div>
        </div>
      </div>

      {/* Contenido / Estados */}
      {query.trim().length < 2 ? (
        <div className="flex flex-col items-center justify-center py-20 text-center gap-3">
          <div className="p-4 rounded-full bg-zinc-900 border border-zinc-800 text-zinc-500">
            <Search size={36} />
          </div>
          <h2 className="text-base font-semibold text-zinc-300">
            Escribe el título de una película, serie o anime
          </h2>
          <p className="text-xs sm:text-sm text-zinc-500 max-w-md">
            Busca en vivo en TMDB y MyAnimeList con filtros de categoría y agregalos en un clic a tu biblioteca.
          </p>
        </div>
      ) : loading && resultados.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-center gap-3">
          <Loader2 size={36} className="animate-spin text-violet-500" />
          <p className="text-sm text-zinc-400">Consultando catálogos online...</p>
        </div>
      ) : hasSearched && resultados.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-center gap-3">
          <div className="p-4 rounded-full bg-zinc-900 border border-zinc-800 text-zinc-500">
            <Film size={36} />
          </div>
          <h2 className="text-base font-semibold text-zinc-300">
            No se encontraron coincidencias para &quot;{query}&quot;
          </h2>
          <p className="text-xs sm:text-sm text-zinc-500 max-w-md">
            Prueba con otro término, verifica la ortografía o cambia el filtro de categoría.
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 pb-20">
          {resultados.map((item) => {
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
      )}
    </div>
  );
}

interface ResultadoCardProps {
  item: ResultadoBusqueda;
  isSaving: boolean;
  onSave: (item: ResultadoBusqueda, estado: "pendiente" | "en_progreso" | "terminada") => void;
  onReanudar?: (item: any) => void;
}

function ResultadoCard({ item, isSaving, onSave, onReanudar }: ResultadoCardProps) {
  const [imgError, setImgError] = useState(false);

  const year = item.fecha_estreno ? item.fecha_estreno.split("-")[0] : null;

  const generosList: string[] = React.useMemo(() => {
    if (!item.generos) return [];
    try {
      const parsed = JSON.parse(item.generos);
      if (Array.isArray(parsed)) return parsed.slice(0, 2);
    } catch {
      // Fallback
    }
    return [];
  }, [item.generos]);

  const rawPoster = item.poster_url;
  const posterSrc = rawPoster
    ? rawPoster.startsWith("http")
      ? rawPoster
      : convertFileSrc(rawPoster)
    : "";

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
            <p className="text-[11px] text-zinc-500 italic line-clamp-1" title={item.titulo_original}>
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
