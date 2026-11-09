import React, { useState, useEffect, useRef, useMemo } from "react";
import { parseCreadores, parseReparto, parsePlataformas } from "./utils/enrichment";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { Info, X } from "lucide-react";
import { CinemaCarousel } from "./components/CinemaCarousel";
import { PosterCard } from "./components/PosterCard";
import { MysteryCard } from "./components/MysteryCard";
import { ContentMetadata } from "./components/ContentMetadata";
import { ActionBar } from "./components/ActionBar";
import { AmbientBackdrop } from "./components/AmbientBackdrop";
import { SynopsisModal } from "./components/SynopsisModal";

interface HoyViewProps {
  recomendacion: any;
  history: any[];
  onSiguiente: (omitir: boolean) => void;
  onAnterior: () => void;
  onUpdateRecomendacion: (updates: any) => void;
  onShowToast: (msg: string) => void;
  loading: boolean;
}

const getMetadata = (jsonStr: string | null) => {
  if (!jsonStr) return null;
  try {
    return JSON.parse(jsonStr);
  } catch (e) {
    return null;
  }
};

export const HoyView: React.FC<HoyViewProps> = ({
  recomendacion,
  history,
  onSiguiente,
  onAnterior,
  onUpdateRecomendacion,
  onShowToast,
  loading
}) => {
  const [isSinopsisOpen, setIsSinopsisOpen] = useState(false);
  const [localProgreso, setLocalProgreso] = useState<any>(null);
  const [localEstado, setLocalEstado] = useState<string | null>(null);

  // Estados para el editor manual de progreso
  const [isModalProgresoOpen, setIsModalProgresoOpen] = useState(false);
  const [editTemporada, setEditTemporada] = useState<number>(1);
  const [editEpisodio, setEditEpisodio] = useState<number>(1);
  const [isFetchingMetadata, setIsFetchingMetadata] = useState(false);
  const [metadataError, setMetadataError] = useState("");

  // Estados de la Ruleta Cinematográfica 3D
  const [isSpinning, setIsSpinning] = useState(false);
  const [frozenContenido, setFrozenContenido] = useState<any>(null);
  const [nextContenido, setNextContenido] = useState<any>(null);
  const [poolItems, setPoolItems] = useState<any[]>([]);

  // Cargar catálogo de respaldo real al montar
  useEffect(() => {
    const fetchPool = async () => {
      try {
        const [progreso, biblioteca, terminadas] = await Promise.all([
          invoke<any[]>("get_en_progreso").catch(() => []),
          invoke<any[]>("get_biblioteca").catch(() => []),
          invoke<any[]>("get_terminadas").catch(() => [])
        ]);
        const allItems = [...progreso, ...biblioteca, ...terminadas]
          .map(it => it?.contenido)
          .filter(c => c && c.poster_url);
        const map = new Map();
        allItems.forEach(c => {
          if (!map.has(c.id)) map.set(c.id, c);
        });
        setPoolItems(Array.from(map.values()));
      } catch (err) {
        console.error("Error al cargar pool para ruleta:", err);
      }
    };
    fetchPool();
  }, []);

  const lastRecomendacionRef = useRef<any>(recomendacion);
  if (recomendacion) {
    lastRecomendacionRef.current = recomendacion;
  }
  const effectiveRecomendacion = recomendacion || (loading ? lastRecomendacionRef.current : null);

  // Capturar la nueva recomendación mientras la ruleta está girando
  useEffect(() => {
    if (recomendacion?.contenido && isSpinning) {
      setNextContenido(recomendacion.contenido);
    }
  }, [recomendacion, isSpinning]);

  const handleStartSpin = () => {
    if (isSpinning) return;
    setIsSinopsisOpen(false);
    setFrozenContenido(effectiveRecomendacion?.contenido || null);
    setNextContenido(null);
    setIsSpinning(true);
    onSiguiente(true);
  };

  const handleSpinComplete = () => {
    setIsSpinning(false);
    setFrozenContenido(null);
  };

  useEffect(() => {
    setLocalEstado(effectiveRecomendacion?.estado || null);
    setLocalProgreso(effectiveRecomendacion?.progreso || null);
  }, [effectiveRecomendacion]);

  const contenido = effectiveRecomendacion?.contenido;
  const displayContenido = isSpinning ? (frozenContenido || contenido) : contenido;

  const parsedGeneros: string[] = (() => {
    if (!displayContenido?.generos) return [];
    if (Array.isArray(displayContenido.generos)) return displayContenido.generos as string[];
    try {
      const parsed = JSON.parse(displayContenido.generos as string);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return typeof displayContenido.generos === "string"
        ? (displayContenido.generos as string).split(",").map((s: string) => s.trim()).filter(Boolean)
        : [];
    }
  })();

  const tipoTexto = displayContenido?.tipo === "MOVIE" ? "Película" : (displayContenido?.es_anime ? "Anime" : "Serie");
  const puntuacion = displayContenido?.puntuacion || displayContenido?.metadata?.puntuacion || displayContenido?.metadata?.vote_average || "";

  const posterUrl = displayContenido?.poster_url?.startsWith("http")
    ? displayContenido.poster_url
    : (displayContenido?.poster_url ? convertFileSrc(displayContenido.poster_url) : "");

  const backdropUrl = displayContenido?.backdrop_url?.startsWith("http")
    ? displayContenido.backdrop_url
    : (displayContenido?.backdrop_url ? convertFileSrc(displayContenido.backdrop_url) : "");

  const parsedCreadores = useMemo(
    () => parseCreadores(displayContenido?.creadores),
    [displayContenido?.creadores]
  );
  const parsedReparto = useMemo(
    () => parseReparto(displayContenido?.reparto),
    [displayContenido?.reparto]
  );
  const parsedPlataformas = useMemo(
    () => parsePlataformas(displayContenido?.plataformas),
    [displayContenido?.plataformas]
  );

  const prevItem = history.length > 0 ? history[history.length - 1] : null;
  const prevPosterUrl = prevItem?.contenido?.poster_url?.startsWith("http")
    ? prevItem.contenido.poster_url
    : (prevItem?.contenido?.poster_url ? convertFileSrc(prevItem.contenido.poster_url) : "");

  const prevTemp = prevItem?.progreso?.temporada_actual || 1;
  const prevEp = prevItem?.progreso?.episodio_actual || 1;
  const prevMeta = getMetadata(prevItem?.progreso?.metadata_temporadas);
  const prevPercent = (() => {
    if (prevMeta && prevMeta[prevTemp] && prevMeta[prevTemp] > 0) {
      return Math.min(100, Math.max(0, Math.round((prevEp / prevMeta[prevTemp]) * 100)));
    }
    return 0;
  })();

  const currentSeason = localProgreso?.temporada_actual || 1;
  const currentEp = localProgreso?.episodio_actual || 1;
  const currentMeta = getMetadata(localProgreso?.metadata_temporadas);
  const percent = (() => {
    if (currentMeta && currentMeta[currentSeason] && currentMeta[currentSeason] > 0) {
      return Math.min(100, Math.max(0, Math.round((currentEp / currentMeta[currentSeason]) * 100)));
    }
    return 0;
  })();

  const anioTexto = (() => {
    if (!displayContenido) return "";
    const startYear = displayContenido.fecha_estreno ? displayContenido.fecha_estreno.substring(0, 4) : "";
    if (displayContenido.tipo === "MOVIE") {
      return startYear;
    }

    if (!startYear) return "";

    const status = displayContenido.status;
    const isEnded = status === "Ended" || status === "Canceled";
    const isOngoing = status === "Returning Series" || status === "In Production";
    const endYear = displayContenido.last_air_date ? displayContenido.last_air_date.substring(0, 4) : "";

    if (isEnded) {
      if (endYear && endYear !== startYear) {
        return `${startYear} - ${endYear}`;
      }
      return startYear;
    }

    if (isOngoing) {
      return `${startYear} - Presente`;
    }

    // Si el status no está especificado o es desconocido, no inferir "Presente"
    if (endYear && endYear !== startYear) {
      return `${startYear} - ${endYear}`;
    }
    return startYear;
  })();

  const duracionTexto = (() => {
    if (!displayContenido) return "";
    if (displayContenido.tipo === "MOVIE") {
      const d = displayContenido.duracion;
      if (typeof d === "number" && d > 0) {
        const hours = Math.floor(d / 60);
        const mins = d % 60;
        if (hours > 0 && mins > 0) {
          return `${hours}h ${mins}m`;
        } else if (hours > 0) {
          return `${hours}h`;
        } else {
          return `${mins}m`;
        }
      }
      return "";
    }
    // Para series o anime, no mostrar duración de película
    return "";
  })();

  const puntuacionTexto = (() => {
    if (puntuacion) {
      const num = Number(puntuacion);
      return isNaN(num) ? String(puntuacion) : num.toFixed(1);
    }
    return "8.6";
  })();

  const isEnProgreso = localEstado === "en_progreso";

  // Auto-enriquecimiento reactivo si el contenido carece de metadata completa
  useEffect(() => {
    const cont = displayContenido;
    if (cont?.id && cont?.tmdb_id) {
      const needsEnrichment =
        !cont.providers_updated_at ||
        (cont.tipo === "MOVIE" && (cont.duracion == null || cont.duracion === 0)) ||
        (cont.tipo !== "MOVIE" && !cont.status);

      if (needsEnrichment && !cont.fetch_recently_failed) {
        invoke<any>("enriquecer_contenido", { id: cont.id })
          .then((enriched) => {
            if (enriched && enriched.id === cont.id) {
              onUpdateRecomendacion({ contenido: enriched });
            }
          })
          .catch((err) => {
            console.error("Error en auto-enriquecimiento de contenido:", err);
          });
      }
    }
  }, [
    displayContenido?.id,
    displayContenido?.tmdb_id,
    displayContenido?.providers_updated_at,
    displayContenido?.duracion,
    displayContenido?.status,
  ]);

  const openProgressModal = async () => {
    if (!displayContenido) return;
    setEditTemporada(localProgreso?.temporada_actual || 1);
    setEditEpisodio(localProgreso?.episodio_actual || 1);
    setIsModalProgresoOpen(true);

    const meta = getMetadata(localProgreso?.metadata_temporadas);
    const isFallback = !meta || Object.keys(meta).length === 0;

    if ((displayContenido.tipo === "TV" || displayContenido.es_anime) && isFallback && (displayContenido.tmdb_id || displayContenido.mal_id)) {
      setIsFetchingMetadata(true);
      setMetadataError("");
      try {
        let newMetaJson = "";
        let retries = 5;
        let success = false;

        while (retries > 0 && !success) {
          try {
            newMetaJson = await invoke<string>("fetch_metadata_temporadas", { id: displayContenido.id });
            success = true;
            setMetadataError("");
          } catch (retryErr: any) {
            const retryErrMsg = typeof retryErr === "string" ? retryErr : (retryErr?.message || "");
            if (retryErrMsg === "fetch_in_progress") {
              await new Promise(resolve => setTimeout(resolve, 500));
              retries--;
            } else {
              setMetadataError(retryErrMsg);
              break;
            }
          }
        }

        if (success) {
          setLocalProgreso((prev: any) => {
            const updated = prev
              ? { ...prev, metadata_temporadas: newMetaJson }
              : { metadata_temporadas: newMetaJson, temporada_actual: 1, episodio_actual: 1 };
            onUpdateRecomendacion({ progreso: updated });
            return updated;
          });
        } else if (!metadataError) {
          setMetadataError("Error: Tiempo de espera agotado al cargar temporadas.");
        }
      } finally {
        setIsFetchingMetadata(false);
      }
    } else if ((displayContenido.tipo === "TV" || displayContenido.es_anime) && isFallback && !displayContenido.tmdb_id && !displayContenido.mal_id) {
      setMetadataError("No se encontró ID de BD. Ingresá manualmente.");
    } else {
      setMetadataError("");
    }
  };

  const saveProgress = async (temp: number, ep: number) => {
    if (!displayContenido) return;
    try {
      await invoke("actualizar_progreso", {
        contenidoId: displayContenido.id,
        temporada: temp,
        episodio: ep
      });
      const newProgreso = {
        ...(localProgreso || {
          id: 0,
          usuario_contenido_id: 0,
          temporada_actual: 1,
          episodio_actual: 1,
          episodio_absoluto_actual: 1,
          metadata_temporadas: ""
        }),
        temporada_actual: temp,
        episodio_actual: ep
      };
      setLocalProgreso(newProgreso);
      onUpdateRecomendacion({ progreso: newProgreso });
      setIsModalProgresoOpen(false);
    } catch (error) {
      console.error("Error saving progress:", error);
    }
  };

  const handleUpdateEstado = async (nuevoEstado: string, autoSkip: boolean = true) => {
    if (!contenido) return;
    setLocalEstado(nuevoEstado);
    onUpdateRecomendacion({ estado: nuevoEstado });

    if (nuevoEstado === "terminada") {
      onShowToast?.(`¡Completaste ${contenido.titulo}!`);
      onSiguiente(false);
    } else if (autoSkip) {
      onSiguiente(localEstado === "pendiente" || localEstado === null);
    }

    invoke("update_estado", { contenidoId: contenido.id, nuevoEstado }).catch(error => {
      console.error(`Error al actualizar estado a ${nuevoEstado}:`, error);
    });

    if (nuevoEstado === "en_progreso" && contenido && (contenido.tipo === "TV" || contenido.es_anime)) {
      if (!localProgreso?.metadata_temporadas && (contenido.tmdb_id || contenido.mal_id)) {
        try {
          const newMetaJson = await invoke<string>("fetch_metadata_temporadas", { id: contenido.id });
          const newProgreso = {
            ...(localProgreso || {
              id: 0,
              usuario_contenido_id: 0,
              temporada_actual: 1,
              episodio_actual: 1,
              episodio_absoluto_actual: 1,
              metadata_temporadas: ""
            }),
            metadata_temporadas: newMetaJson
          };
          setLocalProgreso(newProgreso);
          onUpdateRecomendacion({ progreso: newProgreso });
        } catch (err) {
          console.error("Error fetching metadata on Empezar a ver:", err);
        }
      }
    }
  };

  const handleAddProgress = async () => {
    if (!contenido) return;
    try {
      setLocalEstado("en_progreso");
      onUpdateRecomendacion({ estado: "en_progreso" });
      await invoke("update_estado", { contenidoId: contenido.id, nuevoEstado: "en_progreso" }).catch(e => console.error(e));

      const prevProgreso = localProgreso;
      const nextEpisode = (localProgreso?.episodio_actual || 0) + 1;
      const newProgreso = {
        ...(localProgreso || {
          id: 0,
          usuario_contenido_id: 0,
          temporada_actual: currentSeason,
          episodio_absoluto_actual: nextEpisode,
          metadata_temporadas: ""
        }),
        episodio_actual: nextEpisode
      };

      setLocalProgreso(newProgreso);
      onUpdateRecomendacion({ progreso: newProgreso });

      invoke<{ temporada: number; episodio: number; terminada: boolean; metadata_temporadas?: string | null }>("avanzar_episodio", {
        contenidoId: contenido.id
      })
        .then(result => {
          if (result.terminada) {
            setLocalEstado("terminada");
            onUpdateRecomendacion({ estado: "terminada" });
            onShowToast?.(`¡Completaste ${contenido.titulo}!`);
            onSiguiente(false);
          } else {
            setLocalProgreso((prev: any) => {
              if (!prev) return prev;
              const dbProgreso = {
                ...prev,
                temporada_actual: result.temporada,
                episodio_actual: result.episodio,
                metadata_temporadas: result.metadata_temporadas || prev.metadata_temporadas,
              };
              onUpdateRecomendacion({ progreso: dbProgreso });
              return dbProgreso;
            });
          }
        })
        .catch(err => {
          setLocalProgreso(prevProgreso);
          onUpdateRecomendacion({ progreso: prevProgreso });
          onShowToast?.("No se pudo actualizar el progreso. Intenta nuevamente.");
          console.error("Error al avanzar episodio:", err);
        });
    } catch (error) {
      console.error("Error al actualizar progreso:", error);
    }
  };

  return (
    <div className="relative isolate w-full h-[calc(100vh-3.5rem)] overflow-hidden flex flex-col justify-center items-center px-4 select-none">
      {/* FONDO AMBIENTAL MULTICAPA CON IMAGEN REAL Y TRANSICIÓN SUAVE */}
      <AmbientBackdrop
        backdropUrl={displayContenido?.backdrop_url}
        posterUrl={displayContenido?.poster_url}
      />

      {/* 1. ZONA CENTRAL: CARRUSEL CINEMATOGRÁFICO 3D (ELEVADO Y PROTAGONISTA) */}
      <div className="relative z-10 w-full flex items-center justify-center pt-3 pb-1">
        <CinemaCarousel
          activeId={contenido?.id}
          hasPrevious={!!prevItem && !!prevPosterUrl}
          leftCard={
            <PosterCard
              posterUrl={prevPosterUrl}
              title={prevItem?.contenido?.titulo}
              subtitle={prevItem?.contenido?.titulo_original}
              isCenter={false}
              progress={
                prevItem?.estado === "en_progreso" && prevItem?.contenido?.tipo !== "MOVIE"
                  ? { temp: prevTemp, ep: prevEp, percent: prevPercent }
                  : null
              }
              onClick={onAnterior}
            />
          }
          centerCard={
            <PosterCard
              posterUrl={posterUrl}
              title={displayContenido?.titulo}
              isCenter={true}
              badge={
                isEnProgreso ? (
                  <div className="bg-violet-950/80 border border-violet-500/40 text-violet-300 text-[10px] font-semibold px-2.5 py-0.5 rounded-full backdrop-blur-md flex items-center gap-1.5 shadow-lg">
                    <span className="w-1.5 h-1.5 rounded-full bg-violet-400 animate-pulse" />
                    <span>{displayContenido?.tipo === "MOVIE" ? "EN PROGRESO" : `T${currentSeason} · EN PROGRESO`}</span>
                  </div>
                ) : null
              }
              bottomAction={
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setIsSinopsisOpen(true);
                  }}
                  className="bg-black/60 backdrop-blur-md text-zinc-300 text-xs px-3 py-1 rounded-full border border-white/10 absolute bottom-3 left-1/2 -translate-x-1/2 cursor-pointer hover:border-violet-400 hover:text-white flex items-center gap-1.5 transition-all whitespace-nowrap active:scale-95 shadow-lg select-none focus:outline-none focus-visible:ring-2 focus-visible:ring-violet-400"
                >
                  <Info size={13} className="text-violet-400" />
                  <span>Clic para ver sinopsis</span>
                </button>
              }
            />
          }
          rightCard={
            <MysteryCard onClick={handleStartSpin} />
          }
          onAnterior={onAnterior}
          onSiguiente={handleStartSpin}
          isSpinning={isSpinning}
          currentContent={frozenContenido || contenido}
          nextContent={nextContenido}
          poolItems={poolItems}
          onSpinComplete={handleSpinComplete}
        />
      </div>

      {/* 2. ZONA INFERIOR: METADATA Y BOTONES INFERIORES */}
      <div className="relative z-10 w-full max-w-xl mx-auto text-center flex flex-col items-center gap-1 -mt-1 pb-1">
        <ContentMetadata
          titulo={displayContenido?.titulo || (loading ? "Cargando..." : "Sin recomendación")}
          anio={anioTexto}
          duracion={duracionTexto}
          puntuacion={puntuacionTexto}
          generos={parsedGeneros}
          sinopsis={displayContenido?.sinopsis || displayContenido?.descripcion || "Sin descripción disponible."}
          isEnProgreso={isEnProgreso}
          tipo={tipoTexto}
          progreso={
            isEnProgreso && displayContenido?.tipo !== "MOVIE"
              ? { temp: currentSeason, ep: currentEp, percent }
              : null
          }
          onEditProgress={openProgressModal}
        />
        <ActionBar
          isEnProgreso={isEnProgreso}
          isMovie={displayContenido?.tipo === "MOVIE"}
          onPrimary={isEnProgreso && displayContenido?.tipo !== "MOVIE" ? handleAddProgress : () => handleUpdateEstado("en_progreso", false)}
          onMarkWatched={() => handleUpdateEstado("terminada", true)}
          onSave={() => handleUpdateEstado("pendiente", true)}
          onNext={handleStartSpin}
          disabled={isSpinning}
        />
      </div>

      {/* 4. MODAL DE SINOPSIS EXTENDIDA */}
      <SynopsisModal
        isOpen={isSinopsisOpen}
        onClose={() => setIsSinopsisOpen(false)}
        contenido={displayContenido}
        tipoTexto={tipoTexto}
        generos={parsedGeneros}
        localEstado={localEstado}
        posterUrl={posterUrl}
        backdropUrl={backdropUrl}
        anio={anioTexto}
        duracion={duracionTexto}
        puntuacion={puntuacionTexto}
        progreso={
          isEnProgreso && displayContenido?.tipo !== "MOVIE"
            ? { temp: currentSeason, ep: currentEp, percent }
            : null
        }
        onEditProgress={openProgressModal}
        onPrimaryAction={() => {
          if (displayContenido?.tipo === "MOVIE") {
            handleUpdateEstado("terminada", true);
          } else {
            handleUpdateEstado("en_progreso", false);
          }
        }}
        onAddProgress={handleAddProgress}
        director={displayContenido?.director ?? null}
        creadores={parsedCreadores}
        reparto={parsedReparto}
        totalTemporadas={displayContenido?.total_temporadas ?? null}
        plataformas={parsedPlataformas}
        providersUpdatedAt={displayContenido?.providers_updated_at ?? null}
      />
      {/* 5. MODAL DE EDICIÓN MANUAL DE PROGRESO */}
      {isModalProgresoOpen && displayContenido && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4 animate-in fade-in duration-150 select-none"
          onClick={() => setIsModalProgresoOpen(false)}
        >
          <div
            className="bg-[#11121D] border border-white/10 rounded-2xl w-full max-w-md shadow-2xl p-6 flex flex-col gap-4 text-left"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-lg font-bold text-white line-clamp-1">
                  {displayContenido.titulo}
                </h2>
                <p className="text-xs text-zinc-400">Editar temporada y capítulo</p>
              </div>
              <button
                onClick={() => setIsModalProgresoOpen(false)}
                className="p-1.5 text-zinc-400 hover:text-white bg-white/5 hover:bg-white/10 rounded-full transition-colors cursor-pointer"
                title="Cerrar"
              >
                <X size={18} />
              </button>
            </div>

            {isFetchingMetadata ? (
              <div className="flex flex-col items-center justify-center p-8 gap-2">
                <div className="w-6 h-6 border-2 border-violet-500 border-t-transparent rounded-full animate-spin" />
                <span className="text-zinc-400 text-sm font-medium">Buscando temporadas en TMDB...</span>
              </div>
            ) : (() => {
              const meta = getMetadata(localProgreso?.metadata_temporadas);
              const isFallback = !meta || Object.keys(meta).length === 0;
              const seasons = meta ? Object.keys(meta).map(Number).sort((a, b) => a - b) : [localProgreso?.temporada_actual || 1];
              const currentTotal = meta ? meta[editTemporada.toString()] || 0 : 0;

              return (
                <div className="flex flex-col gap-5">
                  {metadataError && (
                    <div className="text-xs text-amber-400 bg-amber-500/10 p-2.5 rounded-xl border border-amber-500/20">
                      {metadataError}
                    </div>
                  )}

                  {/* Selector de Temporada */}
                  <div className="flex flex-col gap-1.5">
                    <label className="text-xs text-zinc-400 font-semibold uppercase tracking-wider">
                      Temporada
                    </label>
                    {isFallback ? (
                      <div className="flex items-center gap-4">
                        <button
                          onClick={() => setEditTemporada(Math.max(1, editTemporada - 1))}
                          className="px-3 py-1.5 bg-white/10 rounded-lg text-white hover:bg-white/15 cursor-pointer font-bold"
                        >
                          -
                        </button>
                        <span className="text-base font-bold text-white w-12 text-center">
                          {editTemporada}
                        </span>
                        <button
                          onClick={() => setEditTemporada(Math.min(20, editTemporada + 1))}
                          className="px-3 py-1.5 bg-white/10 rounded-lg text-white hover:bg-white/15 cursor-pointer font-bold"
                        >
                          +
                        </button>
                      </div>
                    ) : (
                      <div className="flex gap-2 overflow-x-auto pb-2 scrollbar-thin">
                        {seasons.map((s) => (
                          <button
                            key={s}
                            onClick={() => setEditTemporada(s)}
                            className={`px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all cursor-pointer flex-shrink-0 ${
                              editTemporada === s
                                ? "bg-violet-600 text-white shadow-md shadow-violet-600/30"
                                : "bg-white/5 text-zinc-400 hover:bg-white/10 hover:text-zinc-200 border border-white/5"
                            }`}
                          >
                            Temporada {s}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Selector de Capítulo */}
                  <div className="flex flex-col gap-1.5">
                    <label className="text-xs text-zinc-400 font-semibold uppercase tracking-wider">
                      Capítulo
                    </label>
                    {isFallback ? (
                      <div className="flex flex-col gap-3">
                        <div className="flex items-center gap-4">
                          <button
                            onClick={() => setEditEpisodio(Math.max(1, editEpisodio - 1))}
                            className="px-3 py-1.5 bg-white/10 rounded-lg text-white hover:bg-white/15 cursor-pointer font-bold"
                          >
                            -
                          </button>
                          <span className="text-base font-bold text-white w-12 text-center">
                            {editEpisodio}
                          </span>
                          <button
                            onClick={() => setEditEpisodio(editEpisodio + 1)}
                            className="px-3 py-1.5 bg-white/10 rounded-lg text-white hover:bg-white/15 cursor-pointer font-bold"
                          >
                            +
                          </button>
                        </div>
                        <button
                          onClick={() => saveProgress(editTemporada, editEpisodio)}
                          className="w-full py-2.5 rounded-xl bg-violet-600 hover:bg-violet-500 text-white text-sm font-semibold transition-colors cursor-pointer shadow-lg shadow-violet-600/30"
                        >
                          Guardar Progreso
                        </button>
                      </div>
                    ) : (
                      <div className="grid grid-cols-5 sm:grid-cols-6 gap-2 max-h-56 overflow-y-auto pr-1 scrollbar-thin">
                        {Array.from({ length: currentTotal }).map((_, i) => {
                          const ep = i + 1;
                          const isCurrent =
                            editTemporada === (localProgreso?.temporada_actual || 1) &&
                            ep === (localProgreso?.episodio_actual || 1);
                          return (
                            <button
                              key={ep}
                              onClick={() => saveProgress(editTemporada, ep)}
                              className={`p-2 rounded-xl text-xs font-semibold transition-all cursor-pointer ${
                                isCurrent
                                  ? "bg-violet-600 text-white border border-violet-400 shadow-md shadow-violet-600/40"
                                  : "bg-white/5 text-zinc-300 hover:bg-white/10 hover:text-white border border-white/5"
                              }`}
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

export default HoyView;
