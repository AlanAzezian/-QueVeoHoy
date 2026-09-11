import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ContentGrid, UsuarioContenido } from "./components/ContentGrid";
import { Search, ArrowUpDown } from "lucide-react";

interface BibliotecaViewProps {
  onReanudar?: (item: any) => void;
}

export function BibliotecaView({ onReanudar }: BibliotecaViewProps) {
  const [items, setItems] = useState<UsuarioContenido[]>([]);
  const [terminadasItems, setTerminadasItems] = useState<UsuarioContenido[]>([]);
  const [loading, setLoading] = useState(true);

  const [typeFilter, setTypeFilter] = useState<"todos" | "peliculas" | "series" | "anime">("todos");
  const [stateFilter, setStateFilter] = useState<"pendiente" | "terminada">("terminada");
  const [searchQuery, setSearchQuery] = useState("");
  const [sortOrder, setSortOrder] = useState<"reciente" | "antiguo">("reciente");

  const loadData = async () => {
    setLoading(true);
    try {
      const endpoint = stateFilter === "pendiente" ? "get_biblioteca" : "get_terminadas";
      const [data, termData] = await Promise.all([
        invoke<UsuarioContenido[]>(endpoint),
        invoke<UsuarioContenido[]>("get_terminadas")
      ]);
      setItems(data);
      setTerminadasItems(termData);
    } catch (error) {
      console.error("Error loading biblioteca:", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, [stateFilter]);

  const handleAction = async (item: UsuarioContenido | null, action: "play" | "check" | "add" | "update_progress" | "abandon" | "bulk_abandon", data?: any) => {
    try {
      if (action === "bulk_abandon" && data) {
        await invoke("abandonar_multiples", { ids: data });
      } else if (item) {
        if (action === "play") {
          await invoke("update_estado", { contenidoId: item.contenido.id, nuevoEstado: "en_progreso" });
          if (onReanudar) {
            onReanudar(item);
            return;
          }
        } else if (action === "add") {
          await invoke("update_estado", { contenidoId: item.contenido.id, nuevoEstado: "en_progreso" });
          await invoke("actualizar_progreso", {
            contenidoId: item.contenido.id,
            temporada: item.progreso ? item.progreso.temporada_actual : 1,
            episodio: item.progreso ? item.progreso.episodio_actual + 1 : 1,
          });
        } else if (action === "update_progress" && data) {
          await invoke("actualizar_progreso", {
            contenidoId: item.contenido.id,
            temporada: data.temporada,
            episodio: data.episodio,
          });
        } else if (action === "check") {
          await invoke("update_estado", { contenidoId: item.contenido.id, nuevoEstado: "terminada" });
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

  const filteredItems = items.filter(item => {
    if (typeFilter === "peliculas") return item.contenido.tipo === "MOVIE";
    if (typeFilter === "series") return item.contenido.tipo !== "MOVIE" && !item.contenido.es_anime;
    if (typeFilter === "anime") return item.contenido.es_anime;
    if (searchQuery.trim() !== "") {
      const q = searchQuery.toLowerCase();
      if (!item.contenido.titulo.toLowerCase().includes(q) && !(item.contenido.titulo_original && item.contenido.titulo_original.toLowerCase().includes(q))) {
        return false;
      }
    }
    return true;
  });

  return (
    <div className="w-full max-w-7xl mx-auto py-8 flex flex-col gap-6">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div className="flex items-center gap-4">
          <h1 className="text-2xl font-bold text-white flex items-center gap-3">
            Mi Lista
          </h1>
          <div className="hidden sm:flex items-center gap-3 text-xs font-medium bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-1.5 text-zinc-400">
            <span>Total Terminadas: <span className="text-white">{terminadasItems.length}</span></span>
            <span>•</span>
            <span>Películas: <span className="text-white">{terminadasItems.filter(i => i.contenido.tipo === "MOVIE").length}</span></span>
            <span>•</span>
            <span>Series: <span className="text-white">{terminadasItems.filter(i => i.contenido.tipo !== "MOVIE" && !i.contenido.es_anime).length}</span></span>
            <span>•</span>
            <span>Anime: <span className="text-white">{terminadasItems.filter(i => i.contenido.es_anime).length}</span></span>
          </div>
        </div>
        
        <div className="w-full md:w-64 relative">
          <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
            <Search size={16} className="text-zinc-500" />
          </div>
          <input
            type="text"
            placeholder="Buscar..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-zinc-900 border border-zinc-800 text-white text-sm rounded-xl pl-10 pr-4 py-2 focus:outline-none focus:border-violet-500 transition-colors placeholder-zinc-500"
          />
        </div>
      </div>
      
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-4">
          <div className="flex items-center bg-zinc-900 rounded-xl p-1 border border-zinc-800">
            {(["todos", "peliculas", "series", "anime"] as const).map(t => (
              <button
                key={t}
                onClick={() => setTypeFilter(t)}
                className={`px-3 py-1.5 rounded-lg text-sm font-medium capitalize transition-colors ${typeFilter === t ? "bg-violet-600 text-white" : "text-zinc-400 hover:text-zinc-200"}`}
              >
                {t}
              </button>
            ))}
          </div>

          <div className="flex items-center bg-zinc-900 rounded-xl p-1 border border-zinc-800">
            <button
              onClick={() => setStateFilter("pendiente")}
              className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${stateFilter === "pendiente" ? "bg-zinc-700 text-white" : "text-zinc-400 hover:text-zinc-200"}`}
            >
              Para después
            </button>
            <button
              onClick={() => setStateFilter("terminada")}
              className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${stateFilter === "terminada" ? "bg-zinc-700 text-white" : "text-zinc-400 hover:text-zinc-200"}`}
            >
              Terminadas
            </button>
          </div>
        </div>

        <button
          onClick={() => setSortOrder(prev => prev === "reciente" ? "antiguo" : "reciente")}
          className="flex items-center gap-2 bg-zinc-900 border border-zinc-800 text-zinc-300 hover:text-white px-3 py-2 rounded-xl transition-colors text-sm font-medium ml-auto md:ml-0"
        >
          <ArrowUpDown size={16} className={sortOrder === "reciente" ? "text-violet-400" : "text-zinc-400"} />
          <span className="hidden sm:inline">{sortOrder === "reciente" ? "Más reciente" : "Más antiguo"}</span>
        </button>
      </div>
      
      {(() => {
        const sortedItems = [...filteredItems].sort((a, b) => {
          const dateA = new Date(a.contenido.creado_en || 0).getTime();
          const dateB = new Date(b.contenido.creado_en || 0).getTime();
          return sortOrder === "reciente" ? dateB - dateA : dateA - dateB;
        });

        return (
          <ContentGrid 
            items={sortedItems} 
            onAction={handleAction} 
            emptyMessage="Tu lista está vacía." 
            allowBulkSelection={stateFilter !== 'terminada'}
          />
        );
      })()}
    </div>
  );
}
