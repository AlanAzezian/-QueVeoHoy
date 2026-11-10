import React from "react";
import { Film, Tv, Sparkles } from "lucide-react";

export type TipoContenido = "peliculas" | "series" | "anime";

export interface GeneroConteo {
  nombre: string;
  count: number;
}

export interface FiltrosConfig {
  tipos: TipoContenido[];
  generos: string[];
}

export const TIPOS_CONFIG: {
  id: TipoContenido;
  label: string;
  icon: React.ComponentType<{ size?: number; className?: string }>;
}[] = [
  { id: "peliculas", label: "Películas", icon: Film },
  { id: "series", label: "Series", icon: Tv },
  { id: "anime", label: "Anime", icon: Sparkles },
];

export const GENRES_FALLBACK = [
  "Acción",
  "Aventura",
  "Animación",
  "Comedia",
  "Crimen",
  "Documental",
  "Drama",
  "Familia",
  "Fantasía",
  "Historia",
  "Terror",
  "Música",
  "Misterio",
  "Romance",
  "Ciencia Ficción",
  "Suspenso",
  "Bélico",
  "Western"
];

export function normalizeGenre(str: string): string {
  return str
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .trim();
}

export function parseGeneros(generosRaw?: string | null): string[] {
  if (!generosRaw) return [];
  try {
    const parsed = JSON.parse(generosRaw);
    if (Array.isArray(parsed)) return parsed;
  } catch {
    // Fallback a separado por comas
  }
  if (typeof generosRaw === "string") {
    return generosRaw
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
  }
  return [];
}

/**
 * LÓGICA DE CLASIFICACIÓN DEFINITIVA (ETAPA 5.1.1 & ETAPA 5.6):
 * 1. Sin filtros / Todos / Los tres -> Todo el catálogo
 * 2. Solo Películas -> Películas no anime
 * 3. Solo Series -> Series no anime
 * 4. Solo Anime -> Películas y series anime (no normales)
 * 5. Películas + Series -> Películas y series no anime
 * 6. Películas + Anime -> Películas anime exclusivamente
 * 7. Series + Anime -> Series anime exclusivamente
 */
export function matchTipoSeleccionado(
  item: { tipo: string; es_anime: boolean },
  tipos: TipoContenido[]
): boolean {
  const hasPelis = tipos.includes("peliculas");
  const hasSeries = tipos.includes("series");
  const hasAnime = tipos.includes("anime");

  // Caso 1 u 8: Sin filtros, o los tres seleccionados -> Todo
  if ((!hasPelis && !hasSeries && !hasAnime) || (hasPelis && hasSeries && hasAnime)) {
    return true;
  }

  const esPeli = item.tipo === "MOVIE";
  const esSerie = item.tipo === "TV";
  const esAnime = Boolean(item.es_anime);

  // Caso 2: Solo Películas -> Películas no anime
  if (hasPelis && !hasSeries && !hasAnime) {
    return esPeli && !esAnime;
  }

  // Caso 3: Solo Series -> Series no anime
  if (!hasPelis && hasSeries && !hasAnime) {
    return esSerie && !esAnime;
  }

  // Caso 4: Solo Anime -> Todo el anime (películas y series)
  if (!hasPelis && !hasSeries && hasAnime) {
    return esAnime;
  }

  // Caso 5: Películas + Series -> Películas y series no anime
  if (hasPelis && hasSeries && !hasAnime) {
    return (esPeli || esSerie) && !esAnime;
  }

  // Caso 6: Películas + Anime -> Exclusivamente películas anime
  if (hasPelis && !hasSeries && hasAnime) {
    return esPeli && esAnime;
  }

  // Caso 7: Series + Anime -> Exclusivamente series anime
  if (!hasPelis && hasSeries && hasAnime) {
    return esSerie && esAnime;
  }

  return true;
}

/**
 * Comprueba si los géneros de una obra cumplen con todos los géneros seleccionados (semántica AND).
 */
export function matchesAllGenres(
  itemGenerosRaw: string | null | undefined,
  selectedGeneros: string[]
): boolean {
  if (!selectedGeneros || selectedGeneros.length === 0) return true;
  const itemGeneros = parseGeneros(itemGenerosRaw);
  const normItemGeneros = itemGeneros.map(normalizeGenre);

  return selectedGeneros.every((selectedG) => {
    const normSel = normalizeGenre(selectedG);
    return normItemGeneros.some(
      (g) => g.includes(normSel) || normSel.includes(g)
    );
  });
}

/**
 * Validación integral de filtros (Tipo AND Géneros).
 */
export function matchesFilters(
  item: { tipo: string; es_anime: boolean; generos?: string | null },
  tipos: TipoContenido[],
  generos: string[]
): boolean {
  if (!matchTipoSeleccionado(item, tipos)) {
    return false;
  }
  return matchesAllGenres(item.generos, generos);
}
