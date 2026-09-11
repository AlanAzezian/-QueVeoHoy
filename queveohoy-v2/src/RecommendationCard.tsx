import React, { useState, useEffect } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { Plus, Edit3, X, Check } from "lucide-react";

interface Contenido {
  id: number;
  tipo: string;
  es_anime: boolean;
  titulo: string;
  titulo_original?: string;
  poster_url: string;
  sinopsis: string;
  fecha_estreno: string;
  tmdb_id?: number;
  mal_id?: number;
  creado_en?: string;
  fetch_recently_failed?: boolean;
}

interface RecomendacionHoy {
  contenido: Contenido;
  estado: string | null;
  progreso: any | null;
}

interface RecommendationCardProps {
  recomendacion: RecomendacionHoy | null;
  onSiguiente: (omitir: boolean) => void;
  onUpdateRecomendacion?: (updates: any) => void;
  onShowToast?: (message: string) => void;
}

export const RecommendationCard: React.FC<RecommendationCardProps> = ({
  recomendacion,
  onSiguiente,
  onUpdateRecomendacion,
  onShowToast
}) => {
  if (!recomendacion) {
    return (
      <div className="flex flex-col items-center justify-center w-full h-[600px] bg-zinc-900 rounded-xl">
        <p className="text-zinc-400 text-lg">No hay más recomendaciones para hoy.</p>
      </div>
    );
  }

  const contenido = recomendacion.contenido || {} as any;
  const estado = recomendacion.estado || null;

  // Uses the Tauri asset protocol to load the local image or direct http url
  const posterUrl = contenido?.poster_url?.startsWith("http")
    ? contenido.poster_url
    : (contenido?.poster_url ? convertFileSrc(contenido.poster_url) : "");

  const [localEstado, setLocalEstado] = useState(recomendacion.estado || null);
  const [localProgreso, setLocalProgreso] = useState(recomendacion.progreso);
  const [showEditModal, setShowEditModal] = useState(false);
  const [editTemporada, setEditTemporada] = useState(1);
  const [editEpisodio, setEditEpisodio] = useState(1);
  const [isFetchingMetadata, setIsFetchingMetadata] = useState(false);
  const [metadataError, setMetadataError] = useState("");

  const fetchMetadataSilently = async () => {
    // Avoid double fetching by looking at local state, but we must use a functional pattern if multiple calls happen.
    // However, since we just need the JSON, we can fetch it.
    const meta = getMetadata(localProgreso?.metadata_temporadas);
    const isFallback = !meta || Object.keys(meta).length === 0;
    
    if ((contenido.tipo === "TV" || contenido.es_anime) && isFallback && (contenido.tmdb_id || contenido.mal_id)) {
      try {
        const fetchPromise = invoke<string>("fetch_and_cache_temporadas", { 
          contenidoId: contenido.id, 
          tmdbId: contenido.tmdb_id || null,
          malId: contenido.mal_id || null,
          esAnime: contenido.es_anime || false
        });
        const newMetaJson = await fetchPromise;
        setLocalProgreso((prev: any) => {
          const newP = { ...(prev || {}), metadata_temporadas: newMetaJson };
          if (onUpdateRecomendacion) onUpdateRecomendacion({ progreso: newP });
          return newP;
        });
      } catch (e: any) {
        console.error("Error silencioso buscando temporadas:", e);
      }
    }
  };

  const openEditModal = async () => {
    setEditTemporada(localProgreso?.temporada_actual || 1);
    setEditEpisodio(localProgreso?.episodio_actual || 1);
    setShowEditModal(true);
    
    const meta = getMetadata(localProgreso?.metadata_temporadas);
    const isFallback = !meta || Object.keys(meta).length === 0;
    
    if ((contenido.tipo === "TV" || contenido.es_anime) && isFallback && (contenido.tmdb_id || contenido.mal_id)) {
      setIsFetchingMetadata(true);
      setMetadataError("");
      try {
        const timeoutPromise = new Promise<never>((_, reject) => {
          setTimeout(() => reject(new Error("Timeout: La búsqueda de temporadas tardó demasiado")), 15000);
        });
        
        const fetchPromise = invoke<string>("fetch_and_cache_temporadas", { 
          contenidoId: contenido.id, 
          tmdbId: contenido.tmdb_id || null,
          malId: contenido.mal_id || null,
          esAnime: contenido.es_anime || false
        });

        const newMetaJson = await Promise.race([fetchPromise, timeoutPromise]);
        setLocalProgreso((prev: any) => {
          const newProgreso = { ...(prev || {}), metadata_temporadas: newMetaJson };
          if (onUpdateRecomendacion) onUpdateRecomendacion({ progreso: newProgreso });
          return newProgreso;
        });
      } catch (e: any) {
        console.error("Error al buscar temporadas:", e);
        const errorMessage = typeof e === 'string' ? e : (e?.message || "Error desconocido al buscar temporadas. Usando modo manual.");
        setMetadataError(errorMessage);
      } finally {
        setIsFetchingMetadata(false);
      }
    } else if ((contenido.tipo === "TV" || contenido.es_anime) && isFallback && !contenido.tmdb_id && !contenido.mal_id) {
      setMetadataError("No se encontró ID de base de datos — ingresá manualmente.");
    }
  };

  useEffect(() => {
    setLocalEstado(recomendacion.estado || null);
    setLocalProgreso(recomendacion.progreso);
  }, [recomendacion]);

  useEffect(() => {
    if (showEditModal) {
      setEditTemporada(localProgreso?.temporada_actual || 1);
      setEditEpisodio(localProgreso?.episodio_actual || 1);
    }
  }, [showEditModal, localProgreso]);

  const handleUpdateEstado = (nuevoEstado: string, autoSkip: boolean = true) => {
    setLocalEstado(nuevoEstado);
    if (onUpdateRecomendacion) onUpdateRecomendacion({ estado: nuevoEstado });
    
    if (nuevoEstado === "terminada") {
      if (onShowToast) {
        onShowToast(`¡Completaste ${contenido.titulo}!`);
      }
      onSiguiente(false);
    } else if (autoSkip) {
      onSiguiente(localEstado === "pendiente" || localEstado === null);
    }

    // Fire and forget
    invoke("update_estado", { contenidoId: contenido.id, nuevoEstado }).catch(error => {
      console.error(`Error al actualizar estado a ${nuevoEstado}:`, error);
    });
  };

  const handleAddProgress = async () => {
    try {
      fetchMetadataSilently(); // Fire and forget
      
      // Optimistically assume en_progreso
      setLocalEstado("en_progreso");
      if (onUpdateRecomendacion) onUpdateRecomendacion({ estado: "en_progreso" });
      invoke("update_estado", { contenidoId: contenido.id, nuevoEstado: "en_progreso" }).catch(e => console.error(e));

      const result = await invoke<{temporada: number, episodio: number, terminada: boolean}>("avanzar_episodio", { contenidoId: contenido.id });
      
      if (result.terminada) {
        setLocalEstado("terminada");
        if (onUpdateRecomendacion) onUpdateRecomendacion({ estado: "terminada" });
        if (onShowToast) {
          onShowToast(`¡Completaste ${contenido.titulo}!`);
        }
        onSiguiente(false);
      } else {
        setLocalProgreso((prev: any) => {
          const newProgreso = { 
            ...(prev || {}), 
            temporada_actual: result.temporada, 
            episodio_actual: result.episodio 
          };
          if (onUpdateRecomendacion) {
            onUpdateRecomendacion({ progreso: newProgreso });
          }
          return newProgreso;
        });
      }
    } catch (error) {
      console.error("Error al actualizar progreso:", error);
    }
  };

  const handleSetProgress = async (temporada: number, episodio: number) => {
    try {
      invoke("update_estado", { contenidoId: contenido.id, nuevoEstado: "en_progreso" }).catch(e => console.error(e));
      invoke("actualizar_progreso", {
        contenidoId: contenido.id,
        temporada,
        episodio,
      }).catch(e => console.error(e));
      
      setLocalEstado("en_progreso");
      const newProgreso = { ...localProgreso, temporada_actual: temporada, episodio_actual: episodio };
      setLocalProgreso(newProgreso);
      if (onUpdateRecomendacion) {
        onUpdateRecomendacion({ 
          estado: "en_progreso",
          progreso: newProgreso 
        });
      }
      setShowEditModal(false);
    } catch (error) {
      console.error("Error al setear progreso:", error);
    }
  };

  const getMetadata = (jsonStr?: string) => {
    if (!jsonStr) return null;
    try {
      return JSON.parse(jsonStr) as Record<string, number>;
    } catch {
      return null;
    }
  };

  return (
    <div className="relative flex flex-col w-[350px] max-h-[calc(100vh-80px)] bg-zinc-800 rounded-2xl overflow-y-auto scrollbar-thin shadow-2xl transition-transform duration-300">
      <div className="relative w-full h-[320px] shrink-0 bg-zinc-900">
        <img
          src={posterUrl}
          alt={contenido.titulo}
          className="absolute inset-0 w-full h-full object-cover"
          onError={(e) => {
            const target = e.target as HTMLImageElement;
            if (!target.src.includes("via.placeholder.com")) {
              target.src = `https://via.placeholder.com/350x525/27272a/71717a?text=Sin+Poster`;
            }
          }}
        />
        <div className="absolute inset-0 bg-gradient-to-t from-zinc-900 via-transparent to-transparent" />
        
        {estado === "en_progreso" && (
          <div className="absolute top-4 left-4 bg-blue-600 text-white text-xs font-bold px-3 py-1 rounded-full shadow-lg">
            En progreso
          </div>
        )}
      </div>

      <div className="p-6 flex flex-col gap-4 flex-1">
        <div>
          <h2 className="text-2xl font-bold text-white leading-tight">
            {contenido.titulo}
          </h2>
          <p className="text-sm text-zinc-400 mt-1">
            {contenido.tipo === "MOVIE" ? "Película" : contenido.es_anime ? "Anime" : "Serie"} • {contenido.fecha_estreno?.substring(0, 4)}
          </p>
        </div>

        <p className="text-sm text-zinc-300 line-clamp-4 leading-relaxed">
          {contenido.sinopsis}
        </p>

        <div className="flex flex-col gap-3 mt-auto pt-4">
          {localEstado === "en_progreso" && contenido.tipo !== "MOVIE" ? (
            <div className="flex flex-col gap-2">
              <div className="flex items-center justify-between bg-zinc-900 px-4 py-2 rounded-xl border border-zinc-700">
                <span className="text-sm font-medium text-white flex-1">
                  T{localProgreso?.temporada_actual || 1} · Cap. {localProgreso?.episodio_actual || 1}
                  {(() => {
                    const meta = getMetadata(localProgreso?.metadata_temporadas);
                    const temp = localProgreso?.temporada_actual || 1;
                    if (meta && meta[temp]) {
                      return ` / ${meta[temp]}`;
                    }
                    return "";
                  })()}
                </span>
                <button 
                  onClick={openEditModal}
                  className="p-2 hover:bg-zinc-800 rounded transition-colors text-violet-400"
                >
                  <Edit3 size={16} />
                </button>
                <div className="w-px h-6 bg-zinc-700 mx-1"></div>
                <button 
                  onClick={handleAddProgress}
                  className="p-2 hover:bg-zinc-800 rounded transition-colors text-violet-400 font-bold flex items-center gap-1"
                >
                  <Plus size={16} /> 1
                </button>
              </div>
              {(() => {
                const meta = getMetadata(localProgreso?.metadata_temporadas);
                const temp = localProgreso?.temporada_actual || 1;
                const eps = localProgreso?.episodio_actual || 1;
                if (meta && meta[temp]) {
                  const total = meta[temp];
                  const pct = Math.min(100, Math.max(0, (eps / total) * 100));
                  return (
                    <div className="w-full bg-zinc-800 h-1.5 rounded-full mt-1 overflow-hidden">
                      <div className="bg-violet-500 h-full rounded-full" style={{ width: `${pct}%` }} />
                    </div>
                  );
                }
                return null;
              })()}
              <button 
                onClick={() => handleUpdateEstado("terminada", false)}
                className="w-full bg-zinc-700 hover:bg-zinc-600 text-white font-medium py-3 rounded-xl transition-colors flex justify-center items-center gap-2"
              >
                <Check size={18} /> Ya la vi
              </button>
            </div>
          ) : (
            <button 
              onClick={() => {
                if (localEstado === "en_progreso" && contenido.tipo === "MOVIE") {
                  handleUpdateEstado("terminada", false);
                } else {
                  handleUpdateEstado("en_progreso", false);
                }
              }}
              className="w-full bg-violet-600 hover:bg-violet-500 text-white font-semibold py-3 rounded-xl transition-colors shadow-md"
            >
              {localEstado === "en_progreso" && contenido.tipo === "MOVIE" ? "Marcar como vista" : "Empezar"}
            </button>
          )}
          
          <div className="flex gap-3 mt-1">
            <button 
              onClick={() => handleUpdateEstado("pendiente", true)}
              className="flex-1 bg-zinc-700 hover:bg-zinc-600 text-white font-medium py-3 rounded-xl transition-colors"
            >
              {localEstado === "en_progreso" ? "Pausar" : "Guardar"}
            </button>
            <button
              onClick={() => onSiguiente(localEstado === "pendiente" || localEstado === null)}
              className="flex-1 bg-zinc-700 hover:bg-zinc-600 text-white font-medium py-3 rounded-xl transition-colors"
            >
              Siguiente
            </button>
          </div>
        </div>
      </div>

      {/* Edit Modal for Progress */}
      {showEditModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4" onClick={() => setShowEditModal(false)}>
          <div className="bg-zinc-900 border border-zinc-800 rounded-2xl w-full max-w-md shadow-2xl p-6 flex flex-col gap-4" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-bold text-white line-clamp-1">{contenido.titulo}</h2>
              <button onClick={() => setShowEditModal(false)} className="text-zinc-500 hover:text-white transition-colors">
                <X size={24} />
              </button>
            </div>
            
            {isFetchingMetadata ? (
              <div className="flex justify-center p-8">
                <span className="text-zinc-400 font-medium animate-pulse">Buscando temporadas...</span>
              </div>
            ) : (() => {
              const meta = getMetadata(localProgreso?.metadata_temporadas);
              const isFallback = !meta || Object.keys(meta).length === 0;
              const seasons = meta ? Object.keys(meta).map(Number).sort((a, b) => a - b) : [localProgreso?.temporada_actual || 1];
              const currentTotal = meta ? meta[editTemporada.toString()] || 0 : 0;
              
              return (
                <div className="flex flex-col gap-6">
                  {metadataError && (
                    <div className="text-sm text-yellow-500 bg-yellow-500/10 p-3 rounded-lg border border-yellow-500/20">
                      {metadataError}
                    </div>
                  )}
                  {/* Season Selector */}
                  <div className="flex flex-col gap-2">
                    <label className="text-sm text-zinc-400 font-medium">Temporada</label>
                    {isFallback ? (
                      <div className="flex items-center gap-4">
                        <button onClick={() => setEditTemporada(Math.max(1, editTemporada - 1))} className="p-2 bg-zinc-800 rounded-lg text-white hover:bg-zinc-700">-</button>
                        <span className="text-lg font-bold text-white w-12 text-center">{editTemporada}</span>
                        <button onClick={() => setEditTemporada(Math.min(20, editTemporada + 1))} className="p-2 bg-zinc-800 rounded-lg text-white hover:bg-zinc-700">+</button>
                      </div>
                    ) : (
                      <div className="flex gap-2 overflow-x-auto pb-2 scrollbar-thin">
                        {seasons.map(s => (
                          <button
                            key={s}
                            onClick={() => setEditTemporada(s)}
                            className={`px-4 py-2 rounded-xl text-sm font-semibold transition-colors flex-shrink-0 ${editTemporada === s ? "bg-violet-600 text-white" : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"}`}
                          >
                            Temporada {s}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Episode Selector */}
                  <div className="flex flex-col gap-2">
                    <label className="text-sm text-zinc-400 font-medium">Capítulo</label>
                    {isFallback ? (
                      <div className="flex flex-col gap-4">
                        <div className="flex items-center gap-4">
                           <button onClick={() => setEditEpisodio(Math.max(1, editEpisodio - 1))} className="p-2 bg-zinc-800 rounded-lg text-white hover:bg-zinc-700">-</button>
                           <span className="text-lg font-bold text-white w-12 text-center">{editEpisodio}</span>
                           <button onClick={() => setEditEpisodio(editEpisodio + 1)} className="p-2 bg-zinc-800 rounded-lg text-white hover:bg-zinc-700">+</button>
                        </div>
                        <button
                          onClick={() => handleSetProgress(editTemporada, editEpisodio)}
                          className="w-full py-3 rounded-xl bg-violet-600 text-white font-bold hover:bg-violet-500 transition-colors"
                        >
                          Guardar Progreso
                        </button>
                      </div>
                    ) : (
                      <div className="grid grid-cols-5 sm:grid-cols-6 gap-2 max-h-60 overflow-y-auto pr-2 scrollbar-thin">
                        {Array.from({ length: currentTotal }).map((_, i) => {
                          const ep = i + 1;
                          const isCurrent = editTemporada === (localProgreso?.temporada_actual || 1) && ep === (localProgreso?.episodio_actual || 1);
                          return (
                            <button
                              key={ep}
                              onClick={() => handleSetProgress(editTemporada, ep)}
                              className={`p-2 rounded-lg text-sm font-medium transition-colors ${isCurrent ? "bg-violet-600 text-white border border-violet-400 shadow-[0_0_10px_rgba(139,92,246,0.5)]" : "bg-zinc-800 text-zinc-300 hover:bg-zinc-700 hover:text-white"}`}
                            >
                              {ep}
                            </button>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </div>
              );
            })()}
          </div>
        </div>
      )}
    </div>
  );
};
