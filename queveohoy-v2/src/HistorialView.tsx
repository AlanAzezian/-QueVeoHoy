import { useState, useEffect } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { UsuarioContenido } from "./components/ContentGrid";
import { PlayCircle, Clock, CheckCircle, Bookmark, SkipForward, XCircle } from "lucide-react";

export function HistorialView() {
  const [items, setItems] = useState<UsuarioContenido[]>([]);
  const [loading, setLoading] = useState(true);

  const loadData = async () => {
    setLoading(true);
    try {
      const data = await invoke<UsuarioContenido[]>("get_historial");
      setItems(data);
    } catch (error) {
      console.error("Error loading historial:", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleReanudar = async (item: UsuarioContenido) => {
    try {
      await invoke("update_estado", { contenidoId: item.contenido.id, nuevoEstado: "en_progreso" });
      await loadData();
    } catch (error) {
      console.error("Error updating item in historial:", error);
    }
  };

  const formatFecha = (isoString?: string) => {
    if (!isoString) return "";
    const date = new Date(isoString);
    return date.toLocaleDateString("es-AR", { day: "2-digit", month: "short", year: "numeric" });
  };

  const getActionDetails = (accion?: string) => {
    switch (accion) {
      case "YA_LA_VI":
        return { 
          label: "Terminado", 
          icon: <CheckCircle size={18} className="text-emerald-400" />,
          badgeClass: "bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
        };
      case "PARA_DESPUES":
        return { 
          label: "Guardado", 
          icon: <Bookmark size={18} className="text-purple-400" />,
          badgeClass: "bg-purple-500/10 text-purple-400 border border-purple-500/20"
        };
      case "EMPEZAR_EN_PROGRESO":
        return { 
          label: "En progreso", 
          icon: <PlayCircle size={18} className="text-sky-400" />,
          badgeClass: "bg-sky-500/10 text-sky-400 border border-sky-500/20"
        };
      case "SIGUIENTE":
        return { 
          label: "Omitido", 
          icon: <SkipForward size={18} className="text-amber-400" />,
          badgeClass: "bg-amber-500/10 text-amber-400 border border-amber-500/20"
        };
      case "ABANDONADA":
        return { 
          label: "Abandonada", 
          icon: <XCircle size={18} className="text-rose-400" />,
          badgeClass: "bg-rose-500/10 text-rose-400 border border-rose-500/20"
        };
      case "AJUSTE_PROGRESO":
        return { 
          label: "En progreso", 
          icon: <Clock size={18} className="text-sky-400" />,
          badgeClass: "bg-sky-500/10 text-sky-400 border border-sky-500/20"
        };
      default:
        return { 
          label: "Actividad", 
          icon: <Clock size={18} className="text-zinc-400" />,
          badgeClass: "bg-zinc-800 text-zinc-300 border border-transparent"
        };
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full w-full">
        <div className="w-10 h-10 border-4 border-violet-600 border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  return (
    <div className="w-full max-w-7xl mx-auto py-8">
      <h1 className="text-2xl font-bold text-white mb-6">Bitácora de Actividad</h1>
      
      {items.length === 0 ? (
        <div className="text-center py-20 text-zinc-500">
          Tu historial está vacío. ¡Empieza a ver contenido!
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          {items.map((item, idx) => {
            const { label, icon, badgeClass } = getActionDetails(item.estado);
            return (
              <div key={`${item.contenido.id}-${idx}`} className="flex items-center justify-between bg-zinc-900 border border-zinc-800 rounded-xl p-4 hover:border-zinc-700 transition-colors">
                <div className="flex items-center gap-4">
                  <div className="flex-shrink-0 w-10 h-10 rounded-full bg-zinc-800 flex items-center justify-center">
                    {icon}
                  </div>
                  {item.contenido.poster_url ? (
                    <img src={item.contenido.poster_url.startsWith('http') ? item.contenido.poster_url : convertFileSrc(item.contenido.poster_url)} alt="poster" className="w-10 h-[60px] object-cover rounded-md flex-shrink-0" />
                  ) : (
                    <div className="w-10 h-[60px] bg-zinc-800 rounded-md flex-shrink-0" />
                  )}
                  <div>
                    <h3 className="text-white font-medium line-clamp-1">{item.contenido.titulo}</h3>
                    <div className="flex items-center gap-2 text-sm text-zinc-400 mt-1">
                      <span className="capitalize">{item.contenido.tipo === "MOVIE" ? "Película" : (item.contenido.es_anime ? "Anime" : "Serie")}</span>
                      <span>•</span>
                      <span>{formatFecha(item.contenido.creado_en)}</span>
                    </div>
                  </div>
                </div>
                
                <div className="flex items-center gap-4">
                  <span className={`hidden sm:inline-block text-sm font-medium px-3 py-1 rounded-lg ${badgeClass || "bg-zinc-800 text-zinc-300 border border-transparent"}`}>
                    {label}
                  </span>
                  
                  <button 
                    onClick={() => handleReanudar(item)}
                    className="p-2 bg-zinc-800 hover:bg-violet-600/20 text-zinc-300 hover:text-violet-400 rounded-lg transition-colors"
                    title="Reanudar / En Progreso"
                  >
                    <PlayCircle size={20} />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
