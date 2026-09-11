import React, { useState, useEffect, useRef } from "react";

import { convertFileSrc, invoke } from "@tauri-apps/api/core";

import { X, Check, Star, Bookmark, Play, HelpCircle, Plus, Info, Edit3, ExternalLink } from "lucide-react";
import { openUrl as open } from '@tauri-apps/plugin-opener';

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

  const fetchingPromisesRef = useRef<Map<number, Promise<string>>>(new Map());

  const [isSinopsisOpen, setIsSinopsisOpen] = useState(false);

  // Use `recomendacionActiva` for rendering, delay updates when spinning

  const [recomendacionActiva, setRecomendacionActiva] = useState(recomendacion);

  const [localProgreso, setLocalProgreso] = useState<any>(null);

  const localProgresoRef = useRef(localProgreso);
  useEffect(() => {
    localProgresoRef.current = localProgreso;
  }, [localProgreso]);

  const SLOT_TEXTS = [

    "🎬 Acción & Aventura...",

    "🍿 Comedia Romántica...",

    "⚔ Anime Shonen...",

    "🚀 Ciencia Ficción...",

    "🧠 Thriller Psicológico...",

    "👻 Terror & Suspenso...",

    "🕵️‍♂️ Crimen & Misterio...",

    "✨ Fantasía Épica...",

    "🎭 Drama Intenso...",

    "🤠 Western Moderno..."

  ];

  const [localEstado, setLocalEstado] = useState<string | null>(null);



  // Slot Machine / Orbit State

  const [isSpinning, setIsSpinning] = useState(false);

  const [slotText, setSlotText] = useState(SLOT_TEXTS[0]);

  const [slotSpeed, setSlotSpeed] = useState(60);

  const [orbitItems, setOrbitItems] = useState([

    { id: 0, pos: 0, poster: "" },

    { id: 1, pos: 1, poster: "" },

    { id: 2, pos: 2, poster: "" },

    { id: 3, pos: 3, poster: "" },

    { id: 4, pos: 4, poster: "" },

  ]);

  // Pools de pósters dinámicos

  const [progresoPosters, setProgresoPosters] = useState<string[]>([]);

  const [catalogoPosters, setCatalogoPosters] = useState<string[]>([]);

  const [spinMode, setSpinMode] = useState<"progreso" | "catalogo">("catalogo");

  const [titlesPool, setTitlesPool] = useState<string[]>([]);
  const [isModalProgresoOpen, setIsModalProgresoOpen] = useState(false);
  const [editTemporada, setEditTemporada] = useState<number>(1);
  const [editEpisodio, setEditEpisodio] = useState<number>(1);
  const [isFetchingMetadata, setIsFetchingMetadata] = useState(false);
  const [metadataError, setMetadataError] = useState("");

  const loadingRef = useRef(loading);

  const isFirstMount = useRef(true);

  const isInitialLoadRef = useRef(false);

  const spinCallbackRef = useRef<(() => void) | null>(null);

  // Precargar pools en background

  useEffect(() => {

    const fetchPosters = async () => {

      try {

        const progreso = await invoke<any[]>("get_en_progreso");

        setProgresoPosters(progreso.map(p => p.contenido?.poster_url).filter(Boolean));

        const [terminadas, biblioteca] = await Promise.all([

          invoke<any[]>("get_terminadas").catch(() => []),

          invoke<any[]>("get_biblioteca").catch(() => [])

        ]);

        setCatalogoPosters([

          ...terminadas.map(p => p.contenido?.poster_url),

          ...biblioteca.map(p => p.contenido?.poster_url)

        ].filter(Boolean));
        setTitlesPool([
          ...progreso.map((p: any) => p.contenido?.titulo),
          ...terminadas.map((p: any) => p.contenido?.titulo),
          ...biblioteca.map((p: any) => p.contenido?.titulo)
        ].filter(Boolean));

      } catch (e) {

        console.error("Error fetching poster pools", e);

      }

    };

    fetchPosters();

  }, []);

  useEffect(() => {

    loadingRef.current = loading;

  }, [loading]);

  const startSpin = (isInitialLoad: boolean = false, callback?: () => void) => {

    if (isSpinning) return;

    isInitialLoadRef.current = isInitialLoad;

    spinCallbackRef.current = callback || null;

    setIsSpinning(true);

  };

  const hasLoadedOnceRef = useRef(false);

  useEffect(() => {
    if (recomendacion) hasLoadedOnceRef.current = true;
    if (!recomendacion && loading) {
      startSpin(!hasLoadedOnceRef.current);
    }
  }, [recomendacion, loading]);

  useEffect(() => {
    isFirstMount.current = false;
  }, []);

  // Efecto A: sincroniza SOLO cuando no está girando
  useEffect(() => {
    if (!isSpinning) {
      setRecomendacionActiva(recomendacion);
      setSpinMode(recomendacion?.estado === "en_progreso" ? "progreso" : "catalogo");
    }
  }, [isSpinning, recomendacion]);

  // Efecto B: corre el timer de la ruleta, SOLO depende de isSpinning
  useEffect(() => {
    if (!isSpinning) return;

    let timer: number;

    let duration = 0;

    const startTime = Date.now();

    const isInitial = isInitialLoadRef.current;

    const cycleText = () => {

      const sourceTitles = titlesPool.length > 0 ? titlesPool : SLOT_TEXTS;
      setSlotText(sourceTitles[Math.floor(Math.random() * sourceTitles.length)]);

      setOrbitItems(prev => prev.map(item => {

        let newPoster = item.poster;

        if (item.pos === 4) {

          const sourcePool = spinMode === "progreso" ? progresoPosters : catalogoPosters;

          const historyPool = history.filter(h => h?.contenido?.poster_url).map(h => h.contenido.poster_url);

          const finalPool = sourcePool.length > 0 ? sourcePool : (historyPool.length > 0 ? historyPool : []);

          if (finalPool.length > 0) {

            newPoster = finalPool[Math.floor(Math.random() * finalPool.length)];

          }

        }

        return {

          ...item,

          pos: item.pos === 4 ? 0 : item.pos + 1,

          poster: newPoster

        };

      }));

      let nextDelay = 60;

      if (duration > 1300 && duration <= 1700) {

        nextDelay = 100;

      } else if (duration > 1700) {

        nextDelay = 180;

      }

      setSlotSpeed(nextDelay);

      duration += nextDelay;

      const elapsed = Date.now() - startTime;

      if (isInitial) {

        if (!loadingRef.current || elapsed >= 1500) {

          setIsSpinning(false);

          if (spinCallbackRef.current) {

            spinCallbackRef.current();

            spinCallbackRef.current = null;

          }

          return;

        }

      } else {

        const reachedMinDuration = elapsed >= 2300;

        const reachedMaxDuration = elapsed >= 3000;

        if (reachedMinDuration && (!loadingRef.current || reachedMaxDuration)) {

          setIsSpinning(false);

          if (spinCallbackRef.current) {

            spinCallbackRef.current();

            spinCallbackRef.current = null;

          }

          return;

        }

      }

      timer = window.setTimeout(cycleText, nextDelay);

    };

    cycleText();

  return () => clearTimeout(timer);
  }, [isSpinning]);

  const handleSiguienteLocal = (omitir: boolean) => {
    if (isSpinning || loading) return;
    setSpinMode(recomendacionActiva?.estado === "en_progreso" ? "progreso" : "catalogo");
    startSpin(false);
    onSiguiente(omitir);
  };

  const handleAnteriorLocal = () => {
    console.log("[DEBUG] Clic en Anterior disparado, items en history:", history?.length);
    if (isSpinning) return;
    onAnterior();
  };

  useEffect(() => {
    setLocalEstado(recomendacionActiva?.estado || null);
    setLocalProgreso(recomendacionActiva?.progreso || null);
  }, [recomendacionActiva]);

  const contenido = recomendacionActiva?.contenido;

  // generos viene del backend como JSON string: '["Acción","Drama"]' o texto separado por comas
  const parsedGeneros: string[] = (() => {
    if (!contenido?.generos) return [];
    if (Array.isArray(contenido.generos)) return contenido.generos as string[];
    try {
      const parsed = JSON.parse(contenido.generos as string);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return typeof contenido.generos === 'string'
        ? (contenido.generos as string).split(',').map((s: string) => s.trim()).filter(Boolean)
        : [];
    }
  })();
  const tipoTexto = contenido?.tipo === "MOVIE" ? "Película" : (contenido?.es_anime ? "Anime" : "Serie");
  const generosTexto = parsedGeneros.slice(0, 2).join(', ');
  const puntuacion = contenido?.puntuacion || contenido?.metadata?.puntuacion || contenido?.metadata?.vote_average || "";

  const posterUrl = contenido?.poster_url?.startsWith("http")
    ? contenido.poster_url
    : (contenido?.poster_url ? convertFileSrc(contenido.poster_url) : "");

  const showSlotMachine = isSpinning || (loading && isFirstMount.current);

  const prevItem = history.length > 0 ? history[history.length - 1] : null;
  const prevPosterUrl = prevItem?.contenido?.poster_url?.startsWith("http")
    ? prevItem.contenido.poster_url
    : (prevItem?.contenido?.poster_url ? convertFileSrc(prevItem.contenido.poster_url) : "");

  const handleUpdateEstado = async (nuevoEstado: string, autoSkip: boolean = true) => {
    setLocalEstado(nuevoEstado);
    onUpdateRecomendacion({ estado: nuevoEstado });

    if (nuevoEstado === "terminada") {
      onShowToast?.(`¡Completaste ${contenido.titulo}!`);
      handleSiguienteLocal(false);
    } else if (autoSkip) {
      handleSiguienteLocal(localEstado === "pendiente" || localEstado === null);
    }

    invoke("update_estado", { contenidoId: contenido.id, nuevoEstado }).catch(error => {
      console.error(`Error al actualizar estado a ${nuevoEstado}:`, error);
    });

    if (nuevoEstado === "en_progreso" && contenido && (contenido.tipo === "TV" || contenido.es_anime)) {
      if (!localProgreso?.metadata_temporadas && (contenido.tmdb_id || contenido.mal_id)) {
        setIsFetchingMetadata(true);
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
        } finally {
          setIsFetchingMetadata(false);
        }
      }
    }
  };

  const handleAddProgress = () => {
    if (!contenido) return;
    try {
      setLocalEstado("en_progreso");
      onUpdateRecomendacion({ estado: "en_progreso" });
      invoke("update_estado", { contenidoId: contenido.id, nuevoEstado: "en_progreso" }).catch(e => console.error(e));

      const prevProgreso = localProgreso;

      const currentSeason = localProgreso?.temporada_actual || 1;
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

      invoke<{temporada: number, episodio: number, terminada: boolean}>("avanzar_episodio", { contenidoId: contenido.id })
        .then((result) => {
          if (result.terminada) {
            setLocalEstado("terminada");
            onUpdateRecomendacion({ estado: "terminada" });
            onShowToast?.(`¡Completaste ${contenido.titulo}!`);
            handleSiguienteLocal(false);
          } else {
            setLocalProgreso((prev: any) => {
              if (!prev) return prev;
              const dbProgreso = {
                ...prev,
                temporada_actual: result.temporada,
                episodio_actual: result.episodio
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

  const openProgressModal = async () => {
    if (!recomendacionActiva || !contenido) return;
    setEditTemporada(localProgreso?.temporada_actual || 1);
    setEditEpisodio(localProgreso?.episodio_actual || 1);
    setIsModalProgresoOpen(true);

    const meta = getMetadata(localProgreso?.metadata_temporadas);
    const isFallback = !meta || Object.keys(meta).length === 0;

    if ((contenido.tipo === "TV" || contenido.es_anime) && isFallback && (contenido.tmdb_id || contenido.mal_id)) {
      setIsFetchingMetadata(true);
      setMetadataError("");
      try {
        let newMetaJson = "";
        if (fetchingPromisesRef.current.has(contenido.id)) {
          try {
            newMetaJson = await fetchingPromisesRef.current.get(contenido.id)!;
          } catch (e: any) {
            // Ignore if the cached promise rejected, we will poll
          }
        }
        
        let retries = 5;
        let success = false;
        
        if (newMetaJson) {
           success = true;
        }

        while (retries > 0 && !success) {
          try {
            newMetaJson = await invoke<string>("fetch_metadata_temporadas", { id: contenido.id });
            success = true;
            setMetadataError("");
          } catch (retryErr: any) {
            const retryErrMsg = typeof retryErr === 'string' ? retryErr : (retryErr?.message || "");
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
          setLocalProgreso((prev: any) => 
            prev 
              ? { ...prev, metadata_temporadas: newMetaJson } 
              : { metadata_temporadas: newMetaJson, temporada_actual: 1, episodio_actual: 1 }
          );
        } else if (!metadataError) {
          setMetadataError("Error: Tiempo de espera agotado al cargar temporadas.");
        }
      } finally {
        setIsFetchingMetadata(false);
      }
    } else if ((contenido.tipo === "TV" || contenido.es_anime) && isFallback && !contenido.tmdb_id && !contenido.mal_id) {
      setMetadataError("No se encontró ID de BD. Ingresá manualmente.");
    } else {
      setMetadataError("");
    }
  };

  const saveProgress = async (temp: number, ep: number) => {
    try {
      await invoke("actualizar_progreso", {
        contenidoId: contenido?.id || 0,
        temporada: temp,
        episodio: ep
      });
      const newProgreso = { ...localProgreso, temporada_actual: temp, episodio_actual: ep };
      setLocalProgreso(newProgreso);
      onUpdateRecomendacion({ progreso: newProgreso });
      setIsModalProgresoOpen(false);
    } catch (error) {
      console.error("Error saving progress:", error);
    }
  };

  const renderOrbitItem = (item: { id: number, pos: number, poster: string }) => {

    let transform = "";

    let opacity = 0;

    let zIndex = 0;

    switch (item.pos) {

      case 0:

        transform = "translateX(220%) rotateY(-20deg) translateZ(-60px) scale(0.75)";

        opacity = 0;

        zIndex = 0;

        break;

      case 1:

        transform = "translateX(115%) rotateY(-14deg) translateZ(-40px) scale(0.82)";

        opacity = 0.4;

        zIndex = 10;

        break;

      case 2:

        transform = "translateX(0%) rotateY(0deg) translateZ(0px) scale(1)";

        opacity = 1;

        zIndex = 20;

        break;

      case 3:

        transform = "translateX(-115%) rotateY(14deg) translateZ(-40px) scale(0.82)";

        opacity = 0.4;

        zIndex = 10;

        break;

      case 4:

        transform = "translateX(-220%) rotateY(20deg) translateZ(-60px) scale(0.75)";

        opacity = 0;

        zIndex = 0;

        break;

    }

    const isTeleporting = item.pos === 0;

    let itemPoster = null;

    if (item.poster) {

      itemPoster = item.poster.startsWith("http")

        ? item.poster

        : convertFileSrc(item.poster);

    } else {

      // Fallback a history por seguridad si está vacío

      const historyWithPosters = history.filter(h => h?.contenido?.poster_url);

      if (historyWithPosters.length > 0) {

        const histItem = historyWithPosters[item.id % historyWithPosters.length];

        itemPoster = histItem.contenido.poster_url.startsWith("http")

          ? histItem.contenido.poster_url

          : convertFileSrc(histItem.contenido.poster_url);

      }

    }

    return (

      <div

        key={item.id}

        className="absolute h-[440px] md:h-[480px] aspect-[2/3] rounded-2xl border border-purple-500/30 bg-neutral-900/90 shadow-2xl shadow-purple-900/40 overflow-hidden flex items-center justify-center backdrop-blur-sm"

        style={{

          transform,

          opacity,

          zIndex,

          transition: isTeleporting ? "none" : `all ${slotSpeed}ms linear`,

        }}

      >

        {itemPoster ? (

          <>

            <img src={itemPoster} alt="History Poster" className="w-full h-full object-cover opacity-60" />

            <div className="absolute inset-0 bg-neutral-900/40 backdrop-blur-[2px]" />

            <div className="absolute inset-0 border border-purple-500/30 rounded-2xl" />

          </>

        ) : (

          <>

            <div className="absolute inset-0 bg-neutral-900/90" />

            <div className="absolute inset-0 bg-gradient-to-tr from-purple-900/30 via-transparent to-purple-500/10" />

            <div className="w-2/3 h-2/3 border border-purple-500/20 rounded-xl flex items-center justify-center blur-[1px]">

              <div className="w-1/2 h-1 bg-purple-500/40 shadow-[0_0_15px_rgba(168,85,247,0.6)] rounded-full animate-pulse" />

            </div>

          </>

        )}

      </div>

    );

  };

  return (
    <div className="relative w-full h-[calc(100vh-4rem)] overflow-hidden flex flex-col items-center justify-center">
      {/* BACKDROP AMBIENTAL DINÁMICO */}

      <div className="absolute inset-0 z-0 overflow-hidden pointer-events-none">

        {posterUrl && (

          <img

            key={contenido?.id || "empty"}

            src={posterUrl}

            alt="Ambient backdrop"

            className="w-full h-full object-cover scale-150 blur-3xl opacity-60 animate-in fade-in duration-700 ease-out"

          />

        )}

        {/* Gradiente de contraste para asegurar legibilidad */}

        <div className="absolute inset-0 bg-neutral-950/45 backdrop-blur-[2px]" />

        <div className="absolute inset-0 bg-radial from-transparent via-neutral-950/40 to-neutral-950" />

      </div>

      <div className="relative z-10 flex flex-col items-center justify-center w-full max-w-6xl mx-auto py-4 px-4 h-full">

        {(!recomendacionActiva && !showSlotMachine) ? (

          <div className="flex flex-col items-center justify-center h-full gap-4">

            <p className="text-zinc-400 text-lg">No hay más recomendaciones para hoy.</p>

          </div>

        ) : (

          <>

            {/* BLOQUE SUPERIOR: CARRUSEL DE 3 CARTAS */}

            <div className="relative flex items-center justify-center gap-10 lg:gap-16 w-full max-w-6xl mx-auto overflow-hidden [perspective:1200px] min-h-[500px] shrink-0">

              {/* Tarjeta Izquierda (Anterior / Historial) */}

              <div 

                onClick={prevItem && !showSlotMachine ? handleAnteriorLocal : undefined}
                className={
                  showSlotMachine 
                    ? "flex flex-col items-center shrink-0 origin-right transition-opacity duration-300 opacity-0 pointer-events-none"
                    : "flex flex-col items-center shrink-0 origin-right transition-opacity duration-300 opacity-40 hover:opacity-80 cursor-pointer pointer-events-auto z-30"
                }
                style={{ transform: "rotateY(14deg) translateZ(-40px) scale(0.82)" }}

              >

                <div className="h-[440px] md:h-[480px] aspect-[2/3] rounded-2xl overflow-hidden border border-white/5 shadow-xl bg-neutral-900/50 hover:scale-[1.02] transition-all duration-300">

                  {prevPosterUrl && (

                    <img src={prevPosterUrl} alt="Anterior" className="w-full h-full object-cover" />

                  )}

                </div>

              </div>

              {/* Tarjeta Central (Activa) */}

              <div 

                className={`flex flex-col items-center shrink-0 z-20 origin-center transition-opacity duration-300 ${

                  showSlotMachine ? 'opacity-0 pointer-events-none' : 'opacity-100 shadow-2xl shadow-purple-950/60 animate-in fade-in zoom-in-95 duration-350 ease-out'

                }`}

                style={{ transform: "rotateY(0deg) translateZ(0px) scale(1)" }}

              >

                <div 

                  onClick={() => setIsSinopsisOpen(true)}

                  className="h-[440px] md:h-[480px] aspect-[2/3] object-cover rounded-2xl hover:scale-[1.02] transition-all duration-300 cursor-pointer border border-white/10 overflow-hidden group relative bg-neutral-900 hover:shadow-[0_20px_50px_rgba(109,40,217,0.35)]"

                >

                  <img

                    src={posterUrl}

                    alt={contenido?.titulo}

                    className="w-full h-full object-cover"

                  />

                  <div className="absolute inset-0 bg-black/0 hover:bg-black/40 transition-all duration-300 flex items-center justify-center group-hover:bg-black/40">

                    <div className="bg-black/60 backdrop-blur-md text-white font-medium px-3 py-1.5 rounded-full border border-white/20 transform translate-y-4 group-hover:translate-y-0 transition-all duration-300 opacity-0 group-hover:opacity-100">

                      Clic para ver sinopsis

                    </div>

                  </div>

                  {recomendacionActiva?.estado === "en_progreso" && (

                    <div className="absolute top-4 left-4 bg-black/50 backdrop-blur-md border border-violet-400/30 text-violet-200 text-[11px] font-medium tracking-wide uppercase px-2.5 py-0.5 rounded-full flex items-center gap-1.5 shadow-lg">

                      <span className="w-1.5 h-1.5 rounded-full bg-violet-400 animate-pulse" />

                      En progreso

                    </div>

                  )}

                  {/* Botón Info Flotante (Siempre visible para touch) */}

                  <div className="absolute top-4 right-4 bg-black/50 backdrop-blur-md text-white/80 p-1.5 rounded-full hover:bg-black/80 hover:text-white transition-all">

                    <Info size={20} />

                  </div>

                </div>

              </div>

              {/* Tarjeta Derecha (Mystery Card) */}

              <div 
                onClick={!showSlotMachine ? () => handleSiguienteLocal(true) : undefined}
                className={`flex flex-col items-center shrink-0 origin-left transition-opacity duration-300 ${
                  showSlotMachine ? 'opacity-0 pointer-events-none' : 'opacity-100 cursor-pointer'
                }`}
                style={{ transform: "rotateY(-14deg) translateZ(-40px) scale(0.82)" }}
              >
                <div className="h-[440px] md:h-[480px] aspect-[2/3] backdrop-blur-md rounded-2xl flex flex-col items-center justify-center p-6 shadow-xl hover:scale-[1.02] transition-all duration-300 border border-purple-500/30 bg-neutral-950/60 [box-shadow:inset_0_0_20px_rgba(109,40,217,0.15)]">
                  <div
                    className="w-[80px] h-[80px] rounded-full flex items-center justify-center"
                    style={{
                      border: '1.5px solid rgba(192, 132, 252, 0.55)',
                      background: 'linear-gradient(135deg, rgba(76, 29, 149, 0.65) 0%, rgba(24, 10, 42, 0.88) 100%)',
                      boxShadow: '0 0 28px 6px rgba(147, 51, 234, 0.45), 0 0 16px 3px rgba(168, 85, 247, 0.62), 0 0 8px 1px rgba(192, 132, 252, 0.75), 0 0 4px 0px rgba(245, 243, 255, 0.5), inset 0 0 10px 2px rgba(168, 85, 247, 0.40)'
                    }}
                  >
                    <HelpCircle 
                      className="text-purple-400 w-20 h-20 scale-105" 
                      style={{ 
                        animationDuration: '1000ms',
                        filter: 'drop-shadow(0 0 8px rgba(255, 255, 255, 0.9)) drop-shadow(0 0 16px rgba(192, 132, 252, 0.85))'
                      }} 
                    />
                  </div>
                </div>

              </div>

              {/* EFECTO ORBIT SLIDER 3D (Solo visible durante isSpinning) */}

              {showSlotMachine && (

                <div className="absolute inset-0 flex items-center justify-center pointer-events-none">

                  {orbitItems.map(renderOrbitItem)}

                </div>

              )}

            </div>

            {/* BLOQUE INFERIOR: FICHA TÉCNICA Y BOTONERA */}

            <div className={`max-w-[520px] w-full mx-auto mt-5 p-4 rounded-2xl bg-neutral-900/60 backdrop-blur-xl backdrop-saturate-150 border border-white/5 [box-shadow:inset_0_1px_0_0_rgba(255,255,255,0.14),0_24px_48px_-12px_rgba(0,0,0,0.75)] flex flex-col items-center gap-3 transition-opacity duration-300 ${showSlotMachine ? 'opacity-50 pointer-events-none' : 'opacity-100'}`}>

              <div className="min-h-[3.5rem] flex items-center justify-center w-full px-2">

                <h2 className="text-3xl font-black text-white leading-tight font-display tracking-tight line-clamp-2 text-center">

                  {showSlotMachine ? slotText : contenido?.titulo}

                </h2>

              </div>

              <div className="flex items-center justify-center gap-3 text-sm text-neutral-400 font-medium h-5">
              {contenido && !showSlotMachine ? (
                <>
                  <span>
                    {`${contenido.fecha_estreno ? contenido.fecha_estreno.substring(0, 4) : ''} • ${tipoTexto}${generosTexto ? ' • ' + generosTexto : ''}`}
                  </span>
                  {puntuacion && (
                    <>
                      <span>{" • "}</span>
                      <div className="flex items-center gap-1 text-yellow-500">
                        <Star size={14} fill="currentColor" />
                        <span className="text-white font-bold">{puntuacion}</span>
                      </div>
                    </>
                  )}
                </>
              ) : (
                <span className="animate-pulse">Sincronizando...</span>
              )}
            </div>

              {/* Indicador de Progreso Dinámico */}

              {localEstado === "en_progreso" && contenido?.tipo !== "MOVIE" && !showSlotMachine && (

                <div className="w-full flex flex-col items-center mt-1">

                  <span className="text-sm font-medium text-white mb-1 flex items-center justify-center gap-2">
                    {isFetchingMetadata ? (
                      <span className="animate-pulse">Cargando episodios...</span>
                    ) : (
                      <>
                        T{localProgreso?.temporada_actual || 1} {" · "} Cap. {localProgreso?.episodio_actual || 1}
                        {(() => {
                          const meta = getMetadata(localProgreso?.metadata_temporadas);
                          const temp = localProgreso?.temporada_actual || 1;
                          if (meta && meta[temp]) return ` / ${meta[temp]}`;
                          return "";
                        })()}
                        <button 
                          onClick={(e) => { e.stopPropagation(); openProgressModal(); }} 
                          className="p-1.5 bg-purple-500/10 hover:bg-purple-500/20 border border-purple-500/20 rounded-lg text-purple-400 hover:text-purple-300 transition-all shadow-sm"
                          title="Editar progreso"
                        >
                          <Edit3 className="w-5 h-5"/>
                        </button>
                      </>
                    )}
                  </span>

                  <div className="w-full max-w-xs h-1.5 bg-white/10 rounded-full overflow-hidden mx-auto my-1 relative">

                    <div 

                      className="bg-purple-500 h-full rounded-full transition-all duration-500 ease-out absolute left-0 top-0"

                      style={{ width: (() => {

                        const meta = getMetadata(localProgreso?.metadata_temporadas);

                        const temp = localProgreso?.temporada_actual || 1;

                        const eps = localProgreso?.episodio_actual || 1;

                        if (meta && meta[temp] && meta[temp] > 0) {

                          return `${Math.min(100, Math.max(0, (eps / meta[temp]) * 100))}%`;

                        }

                        return '0%';

                      })()}}

                    />

                  </div>

                </div>

              )}

              {/* Botonera inferior horizontal balanceada en una línea */}

              <div className="flex items-center justify-center gap-2 w-full flex-nowrap mt-2">

                {(!contenido || showSlotMachine) ? (

                  <>

                    <button className="whitespace-nowrap shrink-0 flex-1 px-3 py-1.5 rounded-xl text-xs font-bold flex items-center justify-center gap-2 bg-gradient-to-b from-[#7C3AED] to-[#5B21B6] border-t border-white/25 border-b border-black/30 shadow-lg shadow-purple-900/40 hover:shadow-purple-600/30 active:scale-[0.98] transition-all duration-150 text-white [text-shadow:0_1px_2px_rgba(0,0,0,0.4)]">

                      <Play size={18} fill="currentColor" /> Empezar a ver

                    </button>

                    <button className="whitespace-nowrap shrink-0 flex-none bg-white/10 text-white px-3 py-1.5 rounded-xl text-xs font-medium flex items-center gap-2 border border-white/10">

                      <Bookmark size={18} /> Guardar

                    </button>

                    <button className="whitespace-nowrap shrink-0 flex-none bg-white/5 border border-white/10 text-white font-bold px-3 py-1.5 rounded-xl text-xs flex items-center justify-center gap-2">

                      Siguiente

                    </button>

                  </>

                ) : contenido.tipo === "MOVIE" ? (

                  localEstado === "en_progreso" ? (

                    <>

                      <button 

                        onClick={() => handleUpdateEstado("terminada", true)}

                        className="whitespace-nowrap shrink-0 flex-1 px-3 py-1.5 rounded-xl text-xs font-bold flex items-center justify-center gap-2 bg-gradient-to-b from-[#7C3AED] to-[#5B21B6] border-t border-white/25 border-b border-black/30 shadow-lg shadow-purple-900/40 hover:shadow-purple-600/30 active:scale-[0.98] transition-all duration-150 text-white [text-shadow:0_1px_2px_rgba(0,0,0,0.4)]"

                      >

                        <Check size={18} /> Marcar como vista

                      </button>

                      <button 
                        onClick={() => handleSiguienteLocal(true)}
                        className="whitespace-nowrap shrink-0 flex-none bg-white/5 hover:bg-white/10 active:scale-95 border border-white/10 text-white font-bold px-3 py-1.5 rounded-xl text-xs transition-all duration-200 hover:shadow-lg hover:shadow-purple-600/30 flex items-center justify-center gap-2"
                      >
                        Siguiente
                      </button>
                    </>
                  ) : (
                    <>
                      <button 
                        onClick={() => handleUpdateEstado("en_progreso", false)}
                        className="whitespace-nowrap shrink-0 flex-1 px-3 py-1.5 rounded-xl text-xs font-bold flex items-center justify-center gap-2 bg-gradient-to-b from-[#7C3AED] to-[#5B21B6] border-t border-white/25 border-b border-black/30 shadow-lg shadow-purple-900/40 hover:shadow-purple-600/30 active:scale-[0.98] transition-all duration-150 text-white [text-shadow:0_1px_2px_rgba(0,0,0,0.4)]"
                      >
                        <Play size={18} fill="currentColor" /> Empezar a ver
                      </button>
                      <button 
                        onClick={() => handleUpdateEstado("terminada", true)}
                        className="whitespace-nowrap shrink-0 flex-none bg-white/10 hover:bg-white/15 active:scale-95 text-white px-3 py-1.5 rounded-xl text-xs font-medium transition-all duration-200 flex items-center gap-2 border border-white/10 hover:shadow-lg hover:shadow-purple-600/30"
                      >
                        <Check size={18} /> Ya la vi
                      </button>
                      <button 
                        onClick={() => handleUpdateEstado("pendiente", true)}
                        className="whitespace-nowrap shrink-0 flex-none bg-white/10 hover:bg-white/15 active:scale-95 text-white px-3 py-1.5 rounded-xl text-xs font-medium transition-all duration-200 flex items-center gap-2 border border-white/10 hover:shadow-lg hover:shadow-purple-600/30"
                        title="Guardar"
                      >
                        <Bookmark size={18} /> Guardar
                      </button>
                      <button 
                        onClick={() => handleSiguienteLocal(true)}
                        className="whitespace-nowrap shrink-0 flex-none bg-white/5 hover:bg-white/10 active:scale-95 border border-white/10 text-white font-bold px-3 py-1.5 rounded-xl text-xs transition-all duration-200 hover:shadow-lg hover:shadow-purple-600/30 flex items-center justify-center gap-2"
                      >
                        Siguiente
                      </button>
                    </>
                  )
                ) : localEstado === "en_progreso" ? (
                  <>
                    <button 
                      onClick={handleAddProgress}
                      className="whitespace-nowrap shrink-0 flex-1 px-3 py-1.5 rounded-xl text-xs font-bold flex items-center justify-center gap-2 bg-gradient-to-b from-[#7C3AED] to-[#5B21B6] border-t border-white/25 border-b border-black/30 shadow-lg shadow-purple-900/40 hover:shadow-purple-600/30 active:scale-[0.98] transition-all duration-150 text-white [text-shadow:0_1px_2px_rgba(0,0,0,0.4)]"
                    >
                      <Plus size={18} /> +1 Episodio
                    </button>

                    <button 

                      onClick={() => handleUpdateEstado("pendiente", true)}

                      className="whitespace-nowrap shrink-0 flex-none bg-white/10 hover:bg-white/15 active:scale-95 text-white px-3 py-1.5 rounded-xl text-xs font-medium transition-all duration-200 flex items-center gap-2 border border-white/10 hover:shadow-lg hover:shadow-purple-600/30"

                      title="Guardar"

                    >

                      <Bookmark size={18} /> Guardar

                    </button>

                    <button 

                      onClick={() => handleSiguienteLocal(true)}

                      className="whitespace-nowrap shrink-0 flex-none bg-white/5 hover:bg-white/10 active:scale-95 border border-white/10 text-white font-bold px-3 py-1.5 rounded-xl text-xs transition-all duration-200 hover:shadow-lg hover:shadow-purple-600/30 flex items-center justify-center gap-2"

                    >

                      Siguiente

                    </button>

                  </>

                ) : (

                  <>

                    <button 

                      onClick={() => handleUpdateEstado("en_progreso", false)}

                      className="whitespace-nowrap shrink-0 flex-1 bg-[#6D28D9] hover:bg-[#7C3AED] active:scale-95 text-white font-bold px-3 py-1.5 rounded-xl text-xs transition-all duration-200 shadow-lg shadow-purple-900/50 hover:shadow-purple-600/30 flex items-center justify-center gap-2"

                    >

                      <Play size={18} fill="currentColor" /> Empezar a ver

                    </button>

                    <button 

                      onClick={() => handleUpdateEstado("pendiente", true)}

                      className="whitespace-nowrap shrink-0 flex-none bg-white/10 hover:bg-white/15 active:scale-95 text-white px-3 py-1.5 rounded-xl text-xs font-medium transition-all duration-200 flex items-center gap-2 border border-white/10 hover:shadow-lg hover:shadow-purple-600/30"

                      title="Guardar"

                    >

                      <Bookmark size={18} /> Guardar

                    </button>

                    <button 

                      onClick={() => handleSiguienteLocal(true)}

                      className="whitespace-nowrap shrink-0 flex-none bg-white/5 hover:bg-white/10 active:scale-95 border border-white/10 text-white font-bold px-3 py-1.5 rounded-xl text-xs transition-all duration-200 hover:shadow-lg hover:shadow-purple-600/30 flex items-center justify-center gap-2"

                    >

                      Siguiente

                    </button>

                  </>

                )}

              </div>

            </div>

          </>

        )}

      </div>

      {/* MODAL DE SINOPSIS EXTENDIDA */}

      {isSinopsisOpen && contenido && (

        <div 

          className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-md animate-in fade-in duration-200"

          onClick={() => setIsSinopsisOpen(false)}

        >

          <div 

            className="max-w-xl w-full bg-neutral-900/95 border border-white/10 rounded-3xl p-6 md:p-8 shadow-2xl space-y-6 relative overflow-hidden animate-in fade-in zoom-in-95 slide-in-from-bottom-2 duration-200 ease-out"

            onClick={(e) => e.stopPropagation()}

          >

            <button 

              onClick={() => setIsSinopsisOpen(false)}

              className="absolute top-4 right-4 p-2 text-neutral-400 hover:text-white bg-neutral-800/50 hover:bg-neutral-800 rounded-full transition-colors"

            >

              <X size={20} />

            </button>

            <div>

              <h3 className="text-2xl font-bold text-white mb-1 pr-8">{contenido.titulo}</h3>

              {contenido.titulo_original && contenido.titulo_original !== contenido.titulo && (

                <p className="text-sm text-neutral-400 italic mb-3">{contenido.titulo_original}</p>

              )}

              <div className="flex flex-wrap items-center gap-2 mt-3">

                <span className="px-3 py-1 bg-white/10 rounded-full text-xs font-medium text-white">

                  {contenido.fecha_estreno?.substring(0, 4)}

                </span>

                <span className="px-3 py-1 bg-white/10 rounded-full text-xs font-medium text-white">

                  {contenido.tipo === "MOVIE" ? "Película" : contenido.es_anime ? "Anime" : "Serie"}

                </span>

                {parsedGeneros.map((g) => (
                  <span key={g} className="px-2.5 py-0.5 bg-purple-500/20 border border-purple-500/30 rounded-full text-xs font-medium text-purple-200">
                    {g}
                  </span>
                ))}

                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    const isMovie = contenido.tipo === "MOVIE";
                    const url = isMovie
                      ? `https://letterboxd.com/search/${encodeURIComponent(contenido.titulo)}/`
                      : `https://seriesgraph.com/search?q=${encodeURIComponent(contenido.titulo)}`;
                    open(url);
                  }}
                  className="ml-auto flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium text-neutral-400 hover:text-white hover:bg-white/10 border border-transparent hover:border-white/10 transition-all"
                >
                  <ExternalLink size={14} />
                  {contenido.tipo === "MOVIE" ? "Letterboxd" : "SeriesGraph"}
                </button>

              </div>

            </div>

            <div className="w-full h-px bg-white/10" />

            <p className="text-neutral-300 leading-relaxed text-sm md:text-base max-h-[40vh] overflow-y-auto scrollbar-thin pr-2">

              {contenido.sinopsis}

            </p>

            {localEstado !== "en_progreso" && (

              <button 

                onClick={() => {

                  setIsSinopsisOpen(false);

                  if (contenido.tipo === "MOVIE") {

                    handleUpdateEstado("terminada", true);

                  } else {

                    handleUpdateEstado("en_progreso", true);

                  }

                }}

                className="w-full py-3 rounded-xl font-bold flex items-center justify-center gap-2 bg-gradient-to-b from-[#7C3AED] to-[#5B21B6] border-t border-white/25 border-b border-black/30 shadow-lg shadow-purple-900/40 hover:shadow-purple-600/30 active:scale-[0.98] transition-all duration-150 text-white [text-shadow:0_1px_2px_rgba(0,0,0,0.4)]"
                >
                  {contenido.tipo === "MOVIE" ? (
                    <>
                      <Check size={18} /> Marcar como vista
                    </>
                  ) : (
                    <>
                      <Play size={18} fill="currentColor" /> Empezar a ver
                    </>
                  )}
                </button>
              )}
          </div>
        </div>
      )}

      {/* MODAL DE PROGRESO */}
      {isModalProgresoOpen && contenido && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4" onClick={() => setIsModalProgresoOpen(false)}>
          <div className="bg-zinc-900 border border-zinc-800 rounded-2xl w-full max-w-md shadow-2xl p-6 flex flex-col gap-4" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-bold text-white line-clamp-1">{contenido.titulo}</h2>
              <button onClick={() => setIsModalProgresoOpen(false)} className="text-zinc-500 hover:text-white transition-colors">
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
                          onClick={() => saveProgress(editTemporada, editEpisodio)}
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
                              onClick={() => saveProgress(editTemporada, ep)}
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
export default HoyView;
