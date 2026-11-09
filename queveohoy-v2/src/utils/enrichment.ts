export interface MiembroReparto {
  nombre: string;
  personaje: string;
  orden: number;
}

export interface ProveedorStreaming {
  id: number;
  nombre: string;
  logo_path: string | null;
  tipo: string;
}

export interface PlataformasStreaming {
  flatrate: ProveedorStreaming[];
  rent: ProveedorStreaming[];
  buy: ProveedorStreaming[];
}

export const EMPTY_PLATAFORMAS: PlataformasStreaming = Object.freeze({
  flatrate: [],
  rent: [],
  buy: [],
});

/**
 * Parsea de forma segura el campo JSON de creadores de series.
 * Contempla: null, undefined, "", JSON malformado, arrays con valores inesperados, etc.
 */
export function parseCreadores(raw: string | null | undefined): string[] {
  if (!raw || typeof raw !== "string" || !raw.trim()) {
    return [];
  }
  try {
    const data = JSON.parse(raw);
    if (!Array.isArray(data)) {
      return [];
    }
    return data
      .filter((item): item is string => typeof item === "string")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
  } catch {
    return [];
  }
}

/**
 * Parsea de forma segura el campo JSON de reparto principal (cast).
 * Contempla: null, undefined, "", JSON malformado, arrays mixtos, objetos corruptos, etc.
 */
export function parseReparto(raw: string | null | undefined): MiembroReparto[] {
  if (!raw || typeof raw !== "string" || !raw.trim()) {
    return [];
  }
  try {
    const data = JSON.parse(raw);
    if (!Array.isArray(data)) {
      return [];
    }
    return data
      .filter(
        (item): item is Record<string, unknown> =>
          typeof item === "object" && item !== null && !Array.isArray(item)
      )
      .map((item, index) => {
        const nombre = typeof item.nombre === "string" ? item.nombre.trim() : "";
        const personaje =
          typeof item.personaje === "string" ? item.personaje.trim() : "";
        const orden =
          typeof item.orden === "number" && !Number.isNaN(item.orden)
            ? item.orden
            : index;
        return { nombre, personaje, orden };
      })
      .filter((item) => item.nombre.length > 0);
  } catch {
    return [];
  }
}

/**
 * Parsea de forma segura el campo JSON de plataformas de streaming para Argentina.
 * Contempla: null, undefined, "", JSON malformado, objetos parciales, etc.
 */
export function parsePlataformas(raw: string | null | undefined): PlataformasStreaming {
  if (!raw || typeof raw !== "string" || !raw.trim()) {
    return { flatrate: [], rent: [], buy: [] };
  }
  try {
    const data = JSON.parse(raw);
    if (typeof data !== "object" || data === null || Array.isArray(data)) {
      return { flatrate: [], rent: [], buy: [] };
    }

    const parseList = (list: unknown, defaultTipo: string): ProveedorStreaming[] => {
      if (!Array.isArray(list)) return [];
      return list
        .filter(
          (item): item is Record<string, unknown> =>
            typeof item === "object" && item !== null && !Array.isArray(item)
        )
        .map((item) => {
          const id =
            typeof item.id === "number" && !Number.isNaN(item.id) ? item.id : 0;
          const nombre =
            typeof item.nombre === "string" ? item.nombre.trim() : "";
          const logo_path =
            typeof item.logo_path === "string" && item.logo_path.trim()
              ? item.logo_path.trim()
              : null;
          const tipo =
            typeof item.tipo === "string" && item.tipo.trim()
              ? item.tipo.trim()
              : defaultTipo;
          return { id, nombre, logo_path, tipo };
        })
        .filter((item) => item.nombre.length > 0);
    };

    return {
      flatrate: parseList(data.flatrate, "flatrate"),
      rent: parseList(data.rent, "rent"),
      buy: parseList(data.buy, "buy"),
    };
  } catch {
    return { flatrate: [], rent: [], buy: [] };
  }
}
