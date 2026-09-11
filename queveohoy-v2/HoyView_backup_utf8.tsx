import React, { useState, useEffect, useRef } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { X, Check, Star, Bookmark, Play, HelpCircle, Plus, Info, FastForward } from "lucide-react";

interface HoyViewProps {
  recomendacion: any;
  history: any[];
  onSiguiente: (omitir: boolean) => void;
  onAnterior: () => void;
  onUpdateRecomendacion: (updates: any) => void;
  onShowToast: (msg: string) => void;
  onSaltarEnProgreso?: () => Promise<void>;
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
  onSaltarEnProgreso,
  loading
}) => {
  const [isSinopsisOpen, setIsSinopsisOpen] = useState(false);
  
  // Use `recomendacionActiva` for rendering, delay updates when spinning
  const [recomendacionActiva, setRecomendacionActiva] = useState(recomendacion);
  const [localEstado, setLocalEstado] = useState<string | null>(recomendacion?.estado || null);
  const [localProgreso, setLocalProgreso] = useState<any>(recomendacion?.progreso || null);
  
  // Slot Machine / Orbit State
  const [isSpinning, setIsSpinning] = useState(false);
  const [slotText, setSlotText] = useState("Buscando recomendaciÃ³n...");
  const [slotSpeed, setSlotSpeed] = useState(60);
  const [orbitItems, setOrbitItems] = useState([
    { id: 0, pos: 0 },
    { id: 1, pos: 1 },
    { id: 2, pos: 2 },
    { id: 3, pos: 3 },
    { id: 4, pos: 4 },
  ]);

  const spinTimerRef = useRef<number | null>(null);
  const loadingRef = useRef(loading);
  const isFirstMount = useRef(true);

  const SLOT_TEXTS = [
    "ðŸŽ¬ AcciÃ³n & Aventura...",
    "ðŸ¿ Comedia RomÃ¡ntica...",
    "âš” Anime Shonen...",
    "ðŸš€ Ciencia FicciÃ³n...",
    "ðŸ§  Thriller PsicolÃ³gico...",
    "ðŸ‘» Terror & Suspenso...",
    "ðŸ•µï¸â€â™‚ï¸ Crimen & Misterio...",
    "âœ¨ FantasÃ­a Ã‰pica...",
    "ðŸŽ­ Drama Intenso...",
    "ðŸ¤  Western Moderno..."
  ];

  useEffect(() => {
    loadingRef.current = loading;
  }, [loading]);

  useEffect(() => {
    return () => {
      if (spinTimerRef.current) clearTimeout(spinTimerRef.current);
    };
  }, []);

  const startSpin = (callback?: () => void) => {
    if (isSpinning) return;
    setIsSpinning(true);
    let duration = 0;
    
    const cycleText = () => {
      setSlotText(SLOT_TEXTS[Math.floor(Math.random() * SLOT_TEXTS.length)]);
      
      setOrbitItems(prev => prev.map(item => ({
        ...item,
        pos: item.pos === 4 ? 0 : item.pos + 1
      })));
      
      let nextDelay = 60;
      if (duration > 1300 && duration <= 1700) {
        nextDelay = 100;
      } else if (duration > 1700) {
        nextDelay = 180;
      }
      
      setSlotSpeed(nextDelay);
      duration += nextDelay;
      
      const reachedMinDuration = duration >= 2300;
      const reachedMaxDuration = duration >= 3000;
      
      if (!reachedMinDuration || (loadingRef.current && !reachedMaxDuration)) {
        spinTimerRef.current = window.setTimeout(cycleText, nextDelay);
      } else {
        setIsSpinning(false);
        if (callback) callback();
      }
    };
    
    if (spinTimerRef.current) clearTimeout(spinTimerRef.current);
    cycleText();
  };

  useEffect(() => {
    if (isFirstMount.current) {
      isFirstMount.current = false;
      if (!recomendacion && loading) {
        startSpin();
      }
    }
    // Update active recommendation ONLY when not spinning
    if (!isSpinning) {
      setRecomendacionActiva(recomendacion);
    }
  }, [recomendacion, isSpinning, loading]);

  const handleSiguienteLocal = (omitir: boolean) => {
    if (isSpinning || loading) return;
    startSpin();
    onSiguiente(omitir);
  };

  const handleAnteriorLocal = () => {
    if (isSpinning || loading) return;
    onAnterior();
  };

  useEffect(() => {
    setLocalEstado(recomendacionActiva?.estado || null);
    setLocalProgreso(recomendacionActiva?.progreso || null);
  }, [recomendacionActiva]);

  const contenido = recomendacionActiva?.contenido;
  
  const posterUrl = contenido?.poster_url?.startsWith("http")
    ? contenido.poster_url
    : (contenido?.poster_url ? convertFileSrc(contenido.poster_url) : "");

  const showSlotMachine = isSpinning || (loading && isFirstMount.current);

  const prevItem = history.length > 0 ? history[history.length - 1] : null;
  const prevPosterUrl = prevItem?.contenido?.poster_url?.startsWith("http")
    ? prevItem.contenido.poster_url
    : (prevItem?.contenido?.poster_url ? convertFileSrc(prevItem.contenido.poster_url) : "");

  const handleUpdateEstado = (nuevoEstado: string, autoSkip: boolean = true) => {
    setLocalEstado(nuevoEstado);
    onUpdateRecomendacion({ estado: nuevoEstado });
    
    if (nuevoEstado === "terminada") {
      onShowToast(`Â¡Completaste ${contenido.titulo}!`);
      handleSiguienteLocal(false);
    } else if (autoSkip) {
      handleSiguienteLocal(localEstado === "pendiente" || localEstado === null);
    }

    invoke("update_estado", { contenidoId: contenido.id, nuevoEstado }).catch(error => {
      console.error(`Error al actualizar estado a ${nuevoEstado}:`, error);
    });
  };

  const handleAddProgress = async () => {
    try {
      setLocalEstado("en_progreso");
      onUpdateRecomendacion({ estado: "en_progreso" });
      invoke("update_estado", { contenidoId: contenido.id, nuevoEstado: "en_progreso" }).catch(e => console.error(e));

      const result = await invoke<{temporada: number, episodio: number, terminada: boolean}>("avanzar_episodio", { contenidoId: contenido.id });
      
      if (result.terminada) {
        setLocalEstado("terminada");
        onUpdateRecomendacion({ estado: "terminada" });
        onShowToast(`Â¡Completaste ${contenido.titulo}!`);
        handleSiguienteLocal(false);
      } else {
        const newProgreso = { 
          ...(localProgreso || {}), 
          temporada_actual: result.temporada, 
          episodio_actual: result.episodio 
        };
        setLocalProgreso(newProgreso);
        onUpdateRecomendacion({ progreso: newProgreso });
      }
    } catch (error) {
      console.error("Error al actualizar progreso:", error);
    }
  };

  // Helper para renderizar los items en Ã³rbita
  const renderOrbitItem = (item: { id: number, pos: number }) => {
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
    const historyWithPosters = history.filter(h => h?.contenido?.poster_url);
    if (historyWithPosters.length > 0) {
      const histItem = historyWithPosters[item.id % historyWithPosters.length];
      itemPoster = histItem.contenido.poster_url.startsWith("http")
        ? histItem.contenido.poster_url
        : convertFileSrc(histItem.contenido.poster_url);
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

  const handleSaltarEnProgresoLocal = async () => {
    if (isSpinning || loading || !onSaltarEnProgreso) return;
    if (recomendacionActiva?.estado !== "en_progreso") return;
    
    startSpin();
    try {
      await onSaltarEnProgreso();
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="relative w-full h-[calc(100vh-4rem)] overflow-hidden flex flex-col items-center justify-center">
      {onSaltarEnProgreso && (
        <div className="absolute top-4 right-4 z-50">
          <button 
            onClick={handleSaltarEnProgresoLocal}
            disabled={isSpinning || loading || recomendacionActiva?.estado !== "en_progreso"}
            className={`flex items-center gap-2 px-4 py-2 rounded-full transition-all text-sm font-medium shadow-lg border border-white/10 backdrop-blur-md ${
              recomendacionActiva?.estado === "en_progreso" && !isSpinning && !loading
                ? "bg-neutral-900/80 hover:bg-neutral-800 text-neutral-300 cursor-pointer"
                : "bg-neutral-900/40 text-neutral-500 opacity-40 cursor-not-allowed pointer-events-none"
            }`}
            title={recomendacionActiva?.estado === "en_progreso" ? "Saltar todas las series en progreso" : "Ya estÃ¡s en recomendaciones nuevas"}
          >
            <FastForward size={16} />
            Saltar en progreso
          </button>
        </div>
      )}
      {/* BACKDROP AMBIENTAL DINÃMICO */}
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
            <p className="text-zinc-400 text-lg">No hay mÃ¡s recomendaciones para hoy.</p>
          </div>
        ) : (
          <>
            {/* BLOQUE SUPERIOR: CARRUSEL DE 3 CARTAS */}
            <div className="relative flex items-center justify-center gap-6 lg:gap-10 w-full max-w-6xl mx-auto overflow-hidden [perspective:1200px] min-h-[500px] shrink-0">
              
              {/* Tarjeta Izquierda (Anterior / Historial) */}
              <div 
                onClick={prevItem && !showSlotMachine ? handleAnteriorLocal : undefined}
                className={`flex flex-col items-center shrink-0 origin-right transition-opacity duration-300 ${
                  showSlotMachine ? 'opacity-0 pointer-events-none' : 'opacity-40 hover:opacity-80 cursor-pointer'
                }`}
                style={{ transform: "rotateY(14deg) translateZ(-40px) scale(0.82)" }}
              >
                <div className="h-[440px] md:h-[480px] aspect-[2/3] rounded-2xl overflow-hidden border border-white/5 shadow-xl bg-neutral-900/50">
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
                  className="h-[440px] md:h-[480px] aspect-[2/3] object-cover rounded-2xl hover:scale-[1.02] transition-all duration-300 cursor-pointer border border-white/10 overflow-hidden group relative bg-neutral-900"
                >
                  <img
                    src={posterUrl}
                    alt={contenido?.titulo}
                    className="w-full h-full object-cover"
                  />
                  <div className="absolute inset-0 bg-black/0 hover:bg-black/40 transition-all duration-300 flex items-center justify-center group-hover:bg-black/40">
                    <div className="bg-black/60 backdrop-blur-md text-white font-medium px-4 py-2 rounded-full border border-white/20 transform translate-y-4 group-hover:translate-y-0 transition-all duration-300 opacity-0 group-hover:opacity-100">
                      Clic para ver sinopsis
                    </div>
                  </div>
                  {recomendacionActiva?.estado === "en_progreso" && (
                    <div className="absolute top-4 left-4 bg-purple-600 text-white text-xs font-bold px-3 py-1 rounded-full shadow-lg">
                      En progreso
                    </div>
                  )}
                  {/* BotÃ³n Info Flotante (Siempre visible para touch) */}
                  <div className="absolute top-4 right-4 bg-black/50 backdrop-blur-md text-white/80 p-1.5 rounded-full hover:bg-black/80 hover:text-white transition-all">
                    <Info size={20} />
                  </div>
                </div>
              </div>

              {/* Tarjeta Derecha (Mystery Card) */}
              <div 
                onClick={!showSlotMachine ? () => handleSiguienteLocal(true) : undefined}
                className={`flex flex-col items-center shrink-0 origin-left transition-opacity duration-300 ${
                  showSlotMachine ? 'opacity-0 pointer-events-none' : 'opacity-40 hover:opacity-80 cursor-pointer'
                }`}
                style={{ transform: "rotateY(-14deg) translateZ(-40px) scale(0.82)" }}
              >
                <div className="h-[440px] md:h-[480px] aspect-[2/3] bg-neutral-900/80 backdrop-blur-md border-2 border-dashed border-purple-500/40 hover:border-purple-400/80 transition-colors duration-300 rounded-2xl flex flex-col items-center justify-center p-6 shadow-xl">
                  <HelpCircle className="text-purple-400 w-20 h-20 animate-pulse scale-105" style={{ animationDuration: '1000ms' }} />
                </div>
              </div>

              {/* EFECTO ORBIT SLIDER 3D (Solo visible durante isSpinning) */}
              {showSlotMachine && (
                <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
                  {orbitItems.map(renderOrbitItem)}
                </div>
              )}
            </div>

            {/* BLOQUE INFERIOR: FICHA TÃ‰CNICA Y BOTONERA */}
            <div className={`w-full max-w-xl mx-auto mt-6 p-4 rounded-2xl bg-black/40 border border-white/10 backdrop-blur-md flex flex-col items-center gap-3 transition-opacity duration-300 ${showSlotMachine ? 'opacity-50 pointer-events-none' : 'opacity-100'}`}>
              <div className="min-h-[3.5rem] flex items-center justify-center w-full px-2">
                <h2 className="text-3xl font-black text-white leading-tight font-display tracking-tight line-clamp-2 text-center">
                  {showSlotMachine ? slotText : contenido?.titulo}
                </h2>
              </div>
              
              <div className="flex items-center justify-center gap-3 text-sm text-neutral-400 font-medium h-5">
                {contenido && !showSlotMachine ? (
                  <>
                    <span>{contenido.fecha_estreno?.substring(0, 4)}</span>
                    <span>{" â€¢ "}</span>
                    <span>{contenido.tipo === "MOVIE" ? "PelÃ­cula" : contenido.es_anime ? "Anime" : "Serie"}</span>
                    <span>{" â€¢ "}</span>
                    <div className="flex items-center gap-1 text-yellow-500">
                      <Star size={14} fill="currentColor" />
                      <span className="text-white font-bold">8.5</span>
                    </div>
                  </>
                ) : (
                  <span className="animate-pulse">Sincronizando...</span>
                )}
              </div>

              {/* Indicador de Progreso DinÃ¡mico */}
              {localEstado === "en_progreso" && contenido?.tipo !== "MOVIE" && !showSlotMachine && (
                <div className="w-full flex flex-col items-center mt-1">
                  <span className="text-sm font-medium text-white mb-1">
                    T{localProgreso?.temporada_actual || 1} {" Â· "} Cap. {localProgreso?.episodio_actual || 1}
                    {(() => {
                      const meta = getMetadata(localProgreso?.metadata_temporadas);
                      const temp = localProgreso?.temporada_actual || 1;
                      if (meta && meta[temp]) return ` / ${meta[temp]}`;
                      return "";
                    })()}
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

              {/* Botonera inferior horizontal balanceada en una lÃ­nea */}
              <div className="flex items-center justify-center gap-3 w-full mt-1">
                {(!contenido || showSlotMachine) ? (
                  <>
                    <button className="whitespace-nowrap shrink-0 flex-1 bg-[#6D28D9] text-white font-bold px-4 py-2 rounded-xl text-sm flex items-center justify-center gap-2">
                      <Play size={18} fill="currentColor" /> Empezar a ver
                    </button>
                    <button className="whitespace-nowrap shrink-0 flex-none bg-white/10 text-white px-4 py-2 rounded-xl text-sm font-medium flex items-center gap-2 border border-white/10">
                      <Bookmark size={18} /> Guardar
                    </button>
                    <button className="whitespace-nowrap shrink-0 flex-none bg-white/5 border border-white/10 text-white font-bold px-4 py-2 rounded-xl text-sm flex items-center justify-center gap-2">
                      Siguiente
                    </button>
                  </>
                ) : contenido.tipo === "MOVIE" ? (
                  localEstado === "en_progreso" ? (
                    <>
                      <button 
                        onClick={() => handleUpdateEstado("terminada", true)}
                        className="whitespace-nowrap shrink-0 flex-1 bg-[#6D28D9] hover:bg-[#7C3AED] active:scale-95 text-white font-bold px-4 py-2 rounded-xl text-sm transition-all duration-200 shadow-lg shadow-purple-900/50 hover:shadow-purple-600/30 flex items-center justify-center gap-2"
                      >
                        <Check size={18} /> Marcar como vista
                      </button>
                      <button 
                        onClick={() => handleSiguienteLocal(true)}
                        className="whitespace-nowrap shrink-0 flex-none bg-white/5 hover:bg-white/10 active:scale-95 border border-white/10 text-white font-bold px-4 py-2 rounded-xl text-sm transition-all duration-200 hover:shadow-lg hover:shadow-purple-600/30 flex items-center justify-center gap-2"
                      >
                        Siguiente
                      </button>
                    </>
                  ) : (
                    <>
                      <button 
                        onClick={() => handleUpdateEstado("en_progreso", false)}
                        className="whitespace-nowrap shrink-0 flex-1 bg-[#6D28D9] hover:bg-[#7C3AED] active:scale-95 text-white font-bold px-4 py-2 rounded-xl text-sm transition-all duration-200 shadow-lg shadow-purple-900/50 hover:shadow-purple-600/30 flex items-center justify-center gap-2"
                      >
                        <Play size={18} fill="currentColor" /> Empezar a ver
                      </button>
                      <button 
                        onClick={() => handleUpdateEstado("terminada", true)}
                        className="whitespace-nowrap shrink-0 flex-none bg-white/10 hover:bg-white/15 active:scale-95 text-white px-4 py-2 rounded-xl text-sm font-medium transition-all duration-200 flex items-center gap-2 border border-white/10 hover:shadow-lg hover:shadow-purple-600/30"
                      >
                        <Check size={18} /> Ya la vi
                      </button>
                      <button 
                        onClick={() => handleUpdateEstado("pendiente", true)}
                        className="whitespace-nowrap shrink-0 flex-none bg-white/10 hover:bg-white/15 active:scale-95 text-white px-4 py-2 rounded-xl text-sm font-medium transition-all duration-200 flex items-center gap-2 border border-white/10 hover:shadow-lg hover:shadow-purple-600/30"
                        title="Guardar"
                      >
                        <Bookmark size={18} /> Guardar
                      </button>
                      <button 
                        onClick={() => handleSiguienteLocal(true)}
                        className="whitespace-nowrap shrink-0 flex-none bg-white/5 hover:bg-white/10 active:scale-95 border border-white/10 text-white font-bold px-4 py-2 rounded-xl text-sm transition-all duration-200 hover:shadow-lg hover:shadow-purple-600/30 flex items-center justify-center gap-2"
                      >
                        Siguiente
                      </button>
                    </>
                  )
                ) : localEstado === "en_progreso" ? (
                  <>
                    <button 
                      onClick={handleAddProgress}
                      className="whitespace-nowrap shrink-0 flex-1 bg-[#6D28D9] hover:bg-[#7C3AED] active:scale-95 text-white font-bold px-4 py-2 rounded-xl text-sm transition-all duration-200 shadow-lg shadow-purple-900/50 hover:shadow-purple-600/30 flex items-center justify-center gap-2"
                    >
                      <Plus size={18} /> +1 Episodio
                    </button>
                    <button 
                      onClick={() => handleUpdateEstado("pendiente", true)}
                      className="whitespace-nowrap shrink-0 flex-none bg-white/10 hover:bg-white/15 active:scale-95 text-white px-4 py-2 rounded-xl text-sm font-medium transition-all duration-200 flex items-center gap-2 border border-white/10 hover:shadow-lg hover:shadow-purple-600/30"
                      title="Guardar"
                    >
                      <Bookmark size={18} /> Guardar
                    </button>
                    <button 
                      onClick={() => handleSiguienteLocal(true)}
                      className="whitespace-nowrap shrink-0 flex-none bg-white/5 hover:bg-white/10 active:scale-95 border border-white/10 text-white font-bold px-4 py-2 rounded-xl text-sm transition-all duration-200 hover:shadow-lg hover:shadow-purple-600/30 flex items-center justify-center gap-2"
                    >
                      Siguiente
                    </button>
                  </>
                ) : (
                  <>
                    <button 
                      onClick={() => handleUpdateEstado("en_progreso", false)}
                      className="whitespace-nowrap shrink-0 flex-1 bg-[#6D28D9] hover:bg-[#7C3AED] active:scale-95 text-white font-bold px-4 py-2 rounded-xl text-sm transition-all duration-200 shadow-lg shadow-purple-900/50 hover:shadow-purple-600/30 flex items-center justify-center gap-2"
                    >
                      <Play size={18} fill="currentColor" /> Empezar a ver
                    </button>
                    <button 
                      onClick={() => handleUpdateEstado("pendiente", true)}
                      className="whitespace-nowrap shrink-0 flex-none bg-white/10 hover:bg-white/15 active:scale-95 text-white px-4 py-2 rounded-xl text-sm font-medium transition-all duration-200 flex items-center gap-2 border border-white/10 hover:shadow-lg hover:shadow-purple-600/30"
                      title="Guardar"
                    >
                      <Bookmark size={18} /> Guardar
                    </button>
                    <button 
                      onClick={() => handleSiguienteLocal(true)}
                      className="whitespace-nowrap shrink-0 flex-none bg-white/5 hover:bg-white/10 active:scale-95 border border-white/10 text-white font-bold px-4 py-2 rounded-xl text-sm transition-all duration-200 hover:shadow-lg hover:shadow-purple-600/30 flex items-center justify-center gap-2"
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
                  {contenido.tipo === "MOVIE" ? "PelÃ­cula" : contenido.es_anime ? "Anime" : "Serie"}
                </span>
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
                className="w-full bg-[#6D28D9] hover:bg-[#7C3AED] text-white font-bold py-3 rounded-xl transition-colors shadow-lg shadow-purple-900/50 flex items-center justify-center gap-2"
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
    </div>
  );
};
