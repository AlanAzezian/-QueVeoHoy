import React, { useState, useEffect } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Play, Check, Plus, Edit3, X, Trash2 } from "lucide-react";

export interface Contenido {
  id: number;
  tipo: string;
  es_anime: boolean;
  titulo: string;
  titulo_original?: string;
  poster_url: string;
  backdrop_url?: string | null;
  sinopsis: string;
  fecha_estreno: string;
  tmdb_id?: number;
  mal_id?: number;
  creado_en?: string;
  fetch_recently_failed?: boolean;
  generos?: string | null;
  director?: string | null;
  creadores?: string | null;
  reparto?: string | null;
  total_temporadas?: number | null;
  plataformas?: string | null;
  providers_updated_at?: string | null;
  status?: string | null;
  last_air_date?: string | null;
  duracion?: number | null;
}

export interface ProgresoSerie {
  id: number;
  temporada_actual: number;
  episodio_actual: number;
  episodio_absoluto_actual?: number;
  metadata_temporadas?: string;
}

export interface UsuarioContenido {
  contenido: Contenido;
  estado: string;
  progreso: ProgresoSerie | null;
}

interface ContentGridProps {
  items: UsuarioContenido[];
  onAction: (item: UsuarioContenido | null, action: "play" | "check" | "add" | "update_progress" | "abandon" | "bulk_abandon", data?: any) => void;
  emptyMessage: string;
  allowBulkSelection?: boolean;
  variant?: "poster" | "landscape";
}

export function ContentGrid({ items, onAction, emptyMessage, allowBulkSelection = true, variant = "poster" }: ContentGridProps) {
  const [editingProgress, setEditingProgress] = useState<UsuarioContenido | null>(null);
  const [editTemporada, setEditTemporada] = useState<number>(1);
  const [editEpisodio, setEditEpisodio] = useState<number>(1);
  const [isFetchingMetadata, setIsFetchingMetadata] = useState(false);
  const [metadataError, setMetadataError] = useState("");
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());

  useEffect(() => {
    if (!allowBulkSelection && selectedIds.size > 0) {
      setSelectedIds(new Set());
    }
  }, [allowBulkSelection, selectedIds.size]);

  const toggleSelection = (e: React.MouseEvent, id: number) => {
    e.stopPropagation();
    if (!allowBulkSelection) return;
    const newSet = new Set(selectedIds);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    setSelectedIds(newSet);
  };

  const handleEditClick = async (e: React.MouseEvent, item: UsuarioContenido) => {
    e.stopPropagation();
    
    // Create a local copy to manipulate
    let currentItem = { ...item };
    if (currentItem.progreso) {
      currentItem.progreso = { ...currentItem.progreso };
    }
    
    setEditTemporada(currentItem.progreso?.temporada_actual || 1);
    setEditEpisodio(currentItem.progreso?.episodio_actual || 1);
    setEditingProgress(currentItem);

    const meta = getMetadata(currentItem.progreso?.metadata_temporadas);
    const isFallback = !meta || Object.keys(meta).length === 0;

    if ((currentItem.contenido.tipo === "TV" || currentItem.contenido.es_anime) && isFallback && (currentItem.contenido.tmdb_id || currentItem.contenido.mal_id)) {
      setIsFetchingMetadata(true);
      setMetadataError("");
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        
        const timeoutPromise = new Promise<never>((_, reject) => {
          setTimeout(() => reject(new Error("Timeout: La búsqueda de temporadas tardó demasiado")), 15000);
        });
        
        const fetchPromise = invoke<string>("fetch_metadata_temporadas", { 
          id: currentItem.contenido.id
        });

        const newMetaJson = await Promise.race([fetchPromise, timeoutPromise]);
        
        setEditingProgress(prev => {
          if (!prev) return prev;
          return {
            ...prev,
            progreso: prev.progreso ? { ...prev.progreso, metadata_temporadas: newMetaJson } : null
          };
        });
      } catch (err: any) {
        console.error("Error al buscar temporadas:", err);
        const errorMessage = typeof err === 'string' ? err : (err?.message || "Error desconocido al buscar temporadas. Usando modo manual.");
        setMetadataError(errorMessage);
      } finally {
        setIsFetchingMetadata(false);
      }
    } else if ((currentItem.contenido.tipo === "TV" || currentItem.contenido.es_anime) && isFallback && !currentItem.contenido.tmdb_id && !currentItem.contenido.mal_id) {
      setMetadataError("No se encontró ID de base de datos — ingresá manualmente.");
    } else {
      setMetadataError("");
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

  useEffect(() => {
    if (editingProgress) {
      setEditTemporada(editingProgress.progreso?.temporada_actual || 1);
      setEditEpisodio(editingProgress.progreso?.episodio_actual || 1);
    }
  }, [editingProgress?.contenido.id]);

  if (items.length === 0) {
    return (
      <div className="flex items-center justify-center h-full w-full">
        <p className="text-zinc-500 text-lg">{emptyMessage}</p>
      </div>
    );
  }

  const gridClasses = variant === "landscape"
    ? "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5 w-full pb-20 max-w-7xl mx-auto"
    : "grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4 w-full pb-20 max-w-7xl mx-auto";

  return (
    <div className={gridClasses}>
      {items.map((item) => {
        const posterUrl = item.contenido.poster_url?.startsWith("http")
          ? item.contenido.poster_url
          : (item.contenido.poster_url ? convertFileSrc(item.contenido.poster_url) : "");

        const isSeries = item.contenido.tipo !== "MOVIE";

        if (variant === "landscape") {
          const meta = getMetadata(item.progreso?.metadata_temporadas);
          const currentSeason = item.progreso?.temporada_actual || 1;
          const currentEpisode = item.progreso?.episodio_actual || 1;
          const totalEpisodes = meta && meta[currentSeason] ? meta[currentSeason] : null;
          const progressPct = totalEpisodes
            ? Math.min(100, Math.max(0, (currentEpisode / totalEpisodes) * 100))
            : 0;
          const year = item.contenido.fecha_estreno ? item.contenido.fecha_estreno.substring(0, 4) : "";
          const tipoLabel = item.contenido.tipo === "MOVIE"
            ? "Película"
            : (item.contenido.es_anime ? "Anime" : "Serie");

          return (
            <div
              key={item.contenido.id}
              onClick={() => onAction(item, "play")}
              className="aspect-[16/9] w-full min-h-[170px] max-h-[220px] rounded-2xl overflow-hidden relative border border-white/10 bg-neutral-900/70 backdrop-blur-md hover:border-purple-500/40 transition-all duration-300 shadow-lg shadow-black/40 flex p-3 gap-3.5 items-center group cursor-pointer"
            >
              {/* Capa ambiental de fondo integrada */}
              <div className="absolute inset-0 -z-10 overflow-hidden pointer-events-none">
                {posterUrl && (
                  <img
                    src={posterUrl}
                    alt=""
                    aria-hidden="true"
                    className="w-full h-full object-cover blur-2xl opacity-25 scale-125"
                  />
                )}
                <div className="absolute inset-0 bg-gradient-to-t from-neutral-950 via-neutral-950/80 to-neutral-950/60" />
              </div>

              {/* Checkbox de selección múltiple en esquina superior derecha */}
              {allowBulkSelection && (
                <div
                  className="absolute top-3 right-3 z-20 cursor-pointer"
                  onClick={(e) => toggleSelection(e, item.contenido.id)}
                  title="Seleccionar"
                >
                  <div
                    className={`w-5 h-5 rounded-md border flex items-center justify-center transition-colors ${
                      selectedIds.has(item.contenido.id)
                        ? "bg-violet-600 border-violet-500"
                        : "bg-black/50 border-white/30 hover:border-white/70"
                    }`}
                  >
                    {selectedIds.has(item.contenido.id) && <Check size={13} className="text-white" />}
                  </div>
                </div>
              )}

              {/* Miniatura izquierda (Póster vertical 2:3) */}
              {posterUrl ? (
                <img
                  src={posterUrl}
                  alt={item.contenido.titulo}
                  className="h-full aspect-[2/3] rounded-xl object-cover shrink-0 border border-white/10 shadow-md transition-transform duration-300 group-hover:scale-102"
                  onError={(e) => {
                    const target = e.target as HTMLImageElement;
                    if (!target.src.includes("via.placeholder.com")) {
                      target.src = `https://via.placeholder.com/350x525/27272a/71717a?text=Sin+Poster`;
                    }
                  }}
                />
              ) : (
                <div className="h-full aspect-[2/3] rounded-xl shrink-0 border border-white/10 shadow-md flex items-center justify-center text-xs text-zinc-500 bg-zinc-800/80 font-medium text-center px-2">
                  Sin póster
                </div>
              )}

              {/* Bloque de información derecho */}
              <div className="flex flex-col flex-1 min-w-0 h-full justify-between py-0.5">
                <div className="flex flex-col">
                  <h3
                    className="font-semibold text-white text-base md:text-lg line-clamp-1 tracking-tight pr-7"
                    title={item.contenido.titulo}
                  >
                    {item.contenido.titulo}
                  </h3>

                  <div className="flex items-center gap-1.5 text-neutral-400 text-xs mt-0.5">
                    <span>{tipoLabel}</span>
                    {year && (
                      <>
                        <span className="text-neutral-600">•</span>
                        <span>{year}</span>
                      </>
                    )}
                  </div>

                  {isSeries && item.progreso ? (
                    <div className="text-purple-300 bg-purple-500/15 border border-purple-500/30 px-2.5 py-0.5 rounded-full text-xs font-medium w-fit my-1.5 shadow-sm shadow-purple-950/30">
                      T{currentSeason} · Cap. {currentEpisode}
                      {totalEpisodes ? ` / ${totalEpisodes}` : ""}
                    </div>
                  ) : (
                    <div className="text-purple-300 bg-purple-500/15 border border-purple-500/30 px-2.5 py-0.5 rounded-full text-xs font-medium w-fit my-1.5 shadow-sm shadow-purple-950/30">
                      {item.contenido.tipo === "MOVIE" ? "Película en curso" : "En progreso"}
                    </div>
                  )}
                </div>

                {/* Barra de progreso exterior en el flujo de texto */}
                <div className="w-full h-1.5 bg-white/10 rounded-full overflow-hidden my-2">
                  <div
                    className="h-full rounded-full bg-gradient-to-r from-violet-500 to-fuchsia-500 transition-all duration-500"
                    style={{ width: `${isSeries ? progressPct : 50}%` }}
                  />
                </div>

                {/* Botonera de acciones rápidas integrada */}
                {item.estado !== "terminada" && (
                  <div className="flex items-center justify-between gap-2 pt-0.5">
                    <div className="flex items-center gap-1.5">
                      {isSeries ? (
                        <>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              onAction(item, "play");
                            }}
                            className="flex items-center gap-1.5 bg-violet-600 hover:bg-violet-500 active:scale-95 text-white px-3 py-1.5 rounded-lg text-xs font-semibold transition-all shadow-md shadow-violet-950/50 cursor-pointer"
                            title="Reanudar en Hoy"
                          >
                            <Play size={13} className="fill-current" />
                            <span>Reanudar</span>
                          </button>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              onAction(item, "add");
                            }}
                            className="flex items-center gap-1 bg-white/10 hover:bg-white/20 active:scale-95 text-zinc-100 px-2.5 py-1.5 rounded-lg text-xs font-medium transition-all border border-white/10 cursor-pointer"
                            title="+1 Episodio"
                          >
                            <Plus size={14} />
                            <span>+1 Ep</span>
                          </button>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              handleEditClick(e, item);
                            }}
                            className="p-1.5 bg-white/5 hover:bg-white/15 active:scale-95 text-purple-300 hover:text-purple-200 rounded-lg transition-all border border-white/10 cursor-pointer"
                            title="Editar progreso"
                          >
                            <Edit3 size={14} />
                          </button>
                        </>
                      ) : (
                        <>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              onAction(item, "play");
                            }}
                            className="flex items-center gap-1.5 bg-violet-600 hover:bg-violet-500 active:scale-95 text-white px-3 py-1.5 rounded-lg text-xs font-semibold transition-all shadow-md shadow-violet-950/50 cursor-pointer"
                            title="Reanudar en Hoy"
                          >
                            <Play size={13} className="fill-current" />
                            <span>Reanudar</span>
                          </button>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              onAction(item, "check");
                            }}
                            className="flex items-center gap-1 bg-emerald-600/80 hover:bg-emerald-500 active:scale-95 text-white px-2.5 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer"
                            title="Marcar como vista"
                          >
                            <Check size={14} />
                            <span>Vista</span>
                          </button>
                        </>
                      )}
                    </div>

                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onAction(item, "abandon");
                      }}
                      className="p-1.5 text-neutral-400 hover:text-rose-400 hover:bg-rose-500/10 active:scale-95 rounded-lg transition-all cursor-pointer ml-auto"
                      title="Abandonar"
                    >
                      <Trash2 size={15} />
                    </button>
                  </div>
                )}
              </div>
            </div>
          );
        }

        return (
          <div key={item.contenido.id} className="relative group flex flex-col gap-2">
            <div className="relative w-full aspect-[2/3] rounded-xl overflow-hidden bg-zinc-800 shadow-lg cursor-pointer">
              {posterUrl ? (
                <img 
                  src={posterUrl} 
                  alt={item.contenido.titulo} 
                  className="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105" 
                  onError={(e) => {
                    const target = e.target as HTMLImageElement;
                    if (!target.src.includes("via.placeholder.com")) {
                      target.src = `https://via.placeholder.com/350x525/27272a/71717a?text=Sin+Poster`;
                    }
                  }}
                />
              ) : (
                <div className="flex items-center justify-center w-full h-full text-zinc-600 bg-zinc-800 font-medium">Sin póster</div>
              )}
              
              {allowBulkSelection && (
                <div 
                  className="absolute top-2 left-2 z-20 cursor-pointer"
                  onClick={(e) => toggleSelection(e, item.contenido.id)}
                >
                  <div className={`w-5 h-5 rounded border-2 flex items-center justify-center transition-colors ${selectedIds.has(item.contenido.id) ? "bg-violet-600 border-violet-600" : "bg-black/50 border-white/50 hover:border-white"}`}>
                    {selectedIds.has(item.contenido.id) && <Check size={14} className="text-white" />}
                  </div>
                </div>
              )}

              {item.estado !== "terminada" && (
                <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity duration-200 flex flex-col items-center justify-center gap-3">
                  <button 
                    onClick={(e) => { e.stopPropagation(); onAction(item, "abandon"); }}
                    className="bg-red-600/80 hover:bg-red-500 text-white p-2 rounded-full transition-transform hover:scale-110 shadow-lg absolute top-2 right-2"
                    title="Abandonar"
                  >
                    <Trash2 size={16} />
                  </button>
                  {isSeries ? (
                    <>
                      <button 
                        onClick={() => onAction(item, "play")}
                        className="bg-violet-600 hover:bg-violet-500 text-white p-3 rounded-full transition-transform hover:scale-110 shadow-lg"
                        title="Reanudar"
                      >
                        <Play size={24} className="fill-current" />
                      </button>
                      <button 
                        onClick={() => onAction(item, "add")}
                        className="bg-zinc-700 hover:bg-zinc-600 text-white p-2 rounded-full transition-transform hover:scale-110 shadow-lg"
                        title="+1 Episodio"
                      >
                        <Plus size={20} />
                      </button>
                    </>
                  ) : (
                    <button 
                      onClick={() => onAction(item, "check")}
                      className="bg-violet-600 hover:bg-violet-500 text-white p-3 rounded-full transition-transform hover:scale-110 shadow-lg"
                      title="Marcar como vista"
                    >
                      <Check size={24} />
                    </button>
                  )}
                </div>
              )}

              {/* Badges and Progress */}
              {item.progreso && isSeries && item.estado !== "terminada" && (
                <div className="absolute bottom-0 left-0 right-0 bg-black/80 backdrop-blur-md px-2 py-2 text-xs text-white font-medium shadow-md border-t border-white/10 flex flex-col gap-1">
                  <div className="flex items-center justify-between">
                    <span>
                      T{item.progreso.temporada_actual} · Cap. {item.progreso.episodio_actual}
                      {(() => {
                        const meta = getMetadata(item.progreso.metadata_temporadas);
                        if (meta && meta[item.progreso.temporada_actual]) {
                          return ` / ${meta[item.progreso.temporada_actual]}`;
                        }
                        return "";
                      })()}
                    </span>
                    <button 
                      onClick={(e) => handleEditClick(e, item)}
                      className="p-1 hover:bg-white/20 rounded transition-colors text-violet-400 hover:text-violet-300"
                    >
                      <Edit3 size={14} />
                    </button>
                  </div>
                  {(() => {
                    const meta = getMetadata(item.progreso.metadata_temporadas);
                    if (meta && meta[item.progreso.temporada_actual]) {
                      const total = meta[item.progreso.temporada_actual];
                      const pct = Math.min(100, Math.max(0, (item.progreso.episodio_actual / total) * 100));
                      return (
                        <div className="w-full bg-zinc-700 h-1.5 rounded-full mt-1 overflow-hidden">
                          <div className="bg-violet-500 h-full rounded-full" style={{ width: `${pct}%` }} />
                        </div>
                      );
                    }
                    return null;
                  })()}
                </div>
              )}
            </div>
            
            <h3 className="text-zinc-200 text-sm font-medium line-clamp-2 leading-tight">
              {item.contenido.titulo}
            </h3>
          </div>
        );
      })}

      {/* Floating Action Bar */}
      {selectedIds.size > 0 && (
        <div className="fixed bottom-6 left-1/2 transform -translate-x-1/2 bg-zinc-900 border border-zinc-700 shadow-2xl rounded-2xl px-6 py-4 flex items-center gap-6 z-50">
          <div className="text-white font-medium">
            <span className="text-violet-400 font-bold">{selectedIds.size}</span> seleccionados
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={() => setSelectedIds(new Set())}
              className="text-zinc-400 hover:text-white px-4 py-2 font-medium transition-colors"
            >
              Cancelar
            </button>
            <button
              onClick={() => {
                onAction(null, "bulk_abandon", Array.from(selectedIds));
                setSelectedIds(new Set());
              }}
              className="bg-red-600 hover:bg-red-500 text-white px-6 py-2 rounded-xl font-bold transition-colors flex items-center gap-2 shadow-lg"
            >
              <Trash2 size={18} /> Abandonar / Eliminar
            </button>
          </div>
        </div>
      )}

      {/* Edit Modal */}
      {editingProgress && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4" onClick={() => setEditingProgress(null)}>
          <div className="bg-zinc-900 border border-zinc-800 rounded-2xl w-full max-w-md shadow-2xl p-6 flex flex-col gap-4" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-bold text-white line-clamp-1">{editingProgress.contenido.titulo}</h2>
              <button onClick={() => setEditingProgress(null)} className="text-zinc-500 hover:text-white transition-colors">
                <X size={24} />
              </button>
            </div>
            
            {isFetchingMetadata ? (
              <div className="flex justify-center p-8">
                <span className="text-zinc-400 font-medium animate-pulse">Buscando temporadas...</span>
              </div>
            ) : (() => {
              const meta = getMetadata(editingProgress.progreso?.metadata_temporadas);
              const isFallback = !meta || Object.keys(meta).length === 0;
              const seasons = meta ? Object.keys(meta).map(Number).sort((a, b) => a - b) : [editingProgress.progreso?.temporada_actual || 1];
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
                          onClick={() => {
                            onAction(editingProgress, "update_progress", { temporada: editTemporada, episodio: editEpisodio });
                            setEditingProgress(null);
                          }}
                          className="w-full py-3 rounded-xl bg-violet-600 text-white font-bold hover:bg-violet-500 transition-colors"
                        >
                          Guardar Progreso
                        </button>
                      </div>
                    ) : (
                      <div className="grid grid-cols-5 sm:grid-cols-6 gap-2 max-h-60 overflow-y-auto pr-2 scrollbar-thin">
                        {Array.from({ length: currentTotal }).map((_, i) => {
                          const ep = i + 1;
                          const isCurrent = editTemporada === editingProgress.progreso?.temporada_actual && ep === editingProgress.progreso?.episodio_actual;
                          return (
                            <button
                              key={ep}
                              onClick={() => {
                                onAction(editingProgress, "update_progress", { temporada: editTemporada, episodio: ep });
                                setEditingProgress(null);
                              }}
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
}
