import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ContentGrid, UsuarioContenido } from "./components/ContentGrid";
import { ArrowUpDown } from "lucide-react";

interface EnProgresoViewProps {
  onReanudar?: (item: any) => void;
}

export function EnProgresoView({ onReanudar }: EnProgresoViewProps) {
  const [items, setItems] = useState<UsuarioContenido[]>([]);
  const [loading, setLoading] = useState(true);
  const [sortOrder, setSortOrder] = useState<"reciente" | "antiguo">("reciente");

  const loadData = async () => {
    setLoading(true);
    try {
      const data = await invoke<UsuarioContenido[]>("get_en_progreso");
      setItems(data);
      
      // Auto-fetch missing metadata in background (limited concurrency)
      (async () => {
        const toFetch = data.filter(item => {
          const metaStr = item.progreso?.metadata_temporadas;
          const isFallback = !metaStr || metaStr === "{}" || metaStr === "null";
          const isRecentlyFailed = item.contenido.fetch_recently_failed === true;
          return (item.contenido.tipo === "TV" || item.contenido.es_anime) && isFallback && !isRecentlyFailed && (item.contenido.tmdb_id || item.contenido.mal_id);
        });

        let needsRefresh = false;
        const chunkSize = 3;
        
        for (let i = 0; i < toFetch.length; i += chunkSize) {
          const chunk = toFetch.slice(i, i + chunkSize);
          await Promise.all(chunk.map(async (item) => {
            try {
              await invoke("fetch_and_cache_temporadas", { 
                contenidoId: item.contenido.id, 
                tmdbId: item.contenido.tmdb_id || null,
                malId: item.contenido.mal_id || null,
                esAnime: item.contenido.es_anime || false
              });
              needsRefresh = true;
            } catch (e) {
              console.error("Auto-fetch error for", item.contenido.titulo, e);
            }
          }));
        }

        if (needsRefresh) {
          invoke<UsuarioContenido[]>("get_en_progreso").then(newData => setItems(newData));
        }
      })();

    } catch (error) {
      console.error("Error loading en progreso:", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleAction = async (item: UsuarioContenido | null, action: "play" | "check" | "add" | "update_progress" | "abandon" | "bulk_abandon", data?: any) => {
    try {
      if (action === "bulk_abandon" && data) {
        await invoke("abandonar_multiples", { ids: data });
      } else if (item) {
        if (action === "play" && onReanudar) {
          onReanudar(item);
          return; // No need to reload data if navigating away
        } else if (action === "check") {
          await invoke("update_estado", { contenidoId: item.contenido.id, nuevoEstado: "terminada" });
        } else if (action === "add") {
          await invoke("avanzar_episodio", { contenidoId: item.contenido.id });
        } else if (action === "update_progress" && data) {
          await invoke("actualizar_progreso", {
            contenidoId: item.contenido.id,
            temporada: data.temporada,
            episodio: data.episodio,
          });
        } else if (action === "abandon") {
          await invoke("update_estado", { contenidoId: item.contenido.id, nuevoEstado: "abandonada" });
        }
      }
      
      await loadData();
    } catch (error) {
      console.error("Error updating item:", error);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full w-full">
        <div className="w-10 h-10 border-4 border-violet-600 border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  const sortedItems = [...items].sort((a, b) => {
    const dateA = new Date(a.contenido.creado_en || 0).getTime();
    const dateB = new Date(b.contenido.creado_en || 0).getTime();
    return sortOrder === "reciente" ? dateB - dateA : dateA - dateB;
  });

  return (
    <div className="w-full max-w-7xl mx-auto py-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-white">Siguiendo</h1>
        
        <button
          onClick={() => setSortOrder(prev => prev === "reciente" ? "antiguo" : "reciente")}
          className="flex items-center gap-2 bg-zinc-900 border border-zinc-800 text-zinc-300 hover:text-white px-3 py-2 rounded-xl transition-colors text-sm font-medium"
        >
          <ArrowUpDown size={16} className={sortOrder === "reciente" ? "text-violet-400" : "text-zinc-400"} />
          {sortOrder === "reciente" ? "Más reciente" : "Más antiguo"}
        </button>
      </div>

      <ContentGrid 
        items={sortedItems} 
        onAction={handleAction} 
        emptyMessage="No tienes series o películas en progreso." 
      />
    </div>
  );
}
