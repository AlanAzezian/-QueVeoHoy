import React, { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { EnProgresoView } from "./EnProgresoView";
import { BibliotecaView } from "./BibliotecaView";
import { HistorialView } from "./HistorialView";
import { HoyView } from "./HoyView";
import { Sparkles, PlayCircle, Library, Clock, Search } from "lucide-react";
import "./App.css";

type Tab = "hoy" | "en_progreso" | "biblioteca" | "historial" | "buscar";

class ErrorBoundary extends React.Component<{children: React.ReactNode}, {hasError: boolean, error: any}> {
  constructor(props: {children: React.ReactNode}) {
    super(props);
    this.state = { hasError: false, error: null };
  }
  static getDerivedStateFromError(error: any) {
    return { hasError: true, error };
  }
  render() {
    if (this.state.hasError) {
      return (
        <div className="p-10 flex flex-col items-center justify-center h-full w-full bg-red-900/20 text-white">
          <h2 className="text-2xl font-bold text-red-500 mb-4">Error crítico en la vista</h2>
          <pre className="bg-zinc-950 p-4 rounded-xl text-sm overflow-auto max-w-full">
            {this.state.error?.toString()}
          </pre>
          <button onClick={() => window.location.reload()} className="mt-6 bg-red-600 px-6 py-2 rounded-full font-bold">
            Recargar
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

const ALL_GENRES = ["Acción", "Aventura", "Animación", "Comedia", "Crimen", "Documental", "Drama", "Familia", "Fantasía", "Historia", "Terror", "Música", "Misterio", "Romance", "Ciencia ficción", "Suspense", "Bélica", "Western"];

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("hoy");
  const [recomendacion, setRecomendacion] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  const [history, setHistory] = useState<any[]>([]);
  const [forwardHistory, setForwardHistory] = useState<any[]>([]);
  
  const [isGenreOpen, setIsGenreOpen] = useState(false);
  const [selectedGeneros, setSelectedGeneros] = useState<string[]>([]);
  const popoverRef = useRef<HTMLDivElement>(null);

  // Prevent double fetch in Strict Mode
  const isFetchingRef = useRef(false);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(event.target as Node)) {
        setIsGenreOpen(false);
      }
    };
    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setIsGenreOpen(false);
    };
    if (isGenreOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleEscape);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleEscape);
    };
  }, [isGenreOpen]);

  useEffect(() => {
    if (toastMessage) {
      const timer = setTimeout(() => setToastMessage(null), 2500);
      return () => clearTimeout(timer);
    }
  }, [toastMessage]);

  const loadRecommendation = async (retryCount = 0) => {
    if (isFetchingRef.current && retryCount === 0) return;
    isFetchingRef.current = true;
    if (retryCount === 0) setLoading(true);
    
    try {
      let rec = await invoke("get_recommendation");
      
      // Lógica de filtrado por género
      let localRetry = retryCount;
      while (rec && selectedGeneros.length > 0 && localRetry < 40) {
        let recGeneros: string[] = [];
        const genData = (rec as any).contenido?.generos;
        if (typeof genData === 'string') {
          try {
            const parsed = JSON.parse(genData);
            recGeneros = Array.isArray(parsed) ? parsed : [];
          } catch {
            recGeneros = genData.split(',').map((s: string) => s.trim());
          }
        }
        
        const hasMatch = selectedGeneros.some(g => recGeneros.includes(g));
        if (hasMatch) break;
        
        // No match: omit from session but don't log in bitacora
        await invoke("mark_as_omitted", { 
          id: (rec as any).contenido.id, 
          registrarEnBitacora: false 
        }).catch(() => {});
        
        rec = await invoke("get_recommendation");
        localRetry++;
      }
      
      if (localRetry >= 40 || (!rec && selectedGeneros.length > 0)) {
         setToastMessage("No se encontraron obras con esos géneros en el catálogo");
      }
      
      setRecomendacion(rec);
    } catch (error) {
      console.error("Error loading recommendation:", error);
    } finally {
      if (retryCount === 0) setLoading(false);
      isFetchingRef.current = false;
    }
  };

  useEffect(() => {
    if (activeTab === "hoy" && !recomendacion && !isFetchingRef.current) {
      loadRecommendation();
    }
  }, [activeTab, recomendacion, selectedGeneros]); // Added selectedGeneros

  // Polling for fresh content when catalogo_offline is empty
  useEffect(() => {
    let interval: number | undefined;
    if (activeTab === "hoy" && !loading && !recomendacion) {
      interval = window.setInterval(() => {
        if (!isFetchingRef.current) {
          loadRecommendation();
        }
      }, 3000);
    }
    return () => {
      if (interval) window.clearInterval(interval);
    };
  }, [activeTab, loading, recomendacion]);

  const handleSiguiente = async (registrarEnBitacora: boolean = true) => {
    if (recomendacion?.contenido) {
      try {
        await invoke("mark_as_omitted", { 
          id: recomendacion.contenido.id, 
          registrarEnBitacora 
        }).catch(e => console.error("Error skipping:", e));
        
        if (registrarEnBitacora) {
          setHistory(prev => [...prev, recomendacion]);
        }
        
        if (forwardHistory.length > 0) {
          const nextList = [...forwardHistory];
          const nextItem = nextList.shift();
          setForwardHistory(nextList);
          setRecomendacion(nextItem);
        } else {
          setRecomendacion(null);
          await loadRecommendation();
        }
      } catch (error) {
        console.error("Error skipping recommendation:", error);
      }
    }
  };

  const handleAnterior = () => {
    if (history.length > 0) {
      const prevList = [...history];
      const prevItem = prevList.pop();
      setHistory(prevList);
      
      if (recomendacion) {
        setForwardHistory(prev => [recomendacion, ...prev]);
      }
      setRecomendacion(prevItem);
    }
  };

  const handleUpdateRecomendacion = (updates: any) => {
    setRecomendacion((prev: any) => {
      if (!prev) return prev;
      return { ...prev, ...updates };
    });
  };



  return (
    <main className="h-screen w-screen overflow-hidden bg-zinc-950 flex flex-col">
      {/* Global Toast Modal */}
      {toastMessage && (
        <div className="fixed top-8 left-1/2 transform -translate-x-1/2 z-[100] bg-zinc-900 border border-violet-500/50 shadow-[0_0_20px_rgba(139,92,246,0.3)] rounded-full px-6 py-3 flex items-center gap-3 animate-in slide-in-from-top-4 fade-in duration-300">
          <div className="text-2xl">🎉</div>
          <div className="font-bold text-white text-sm">{toastMessage}</div>
        </div>
      )}

      {/* Header / Navigation */}
      <header className="h-16 px-6 flex items-center justify-between w-full fixed top-0 left-0 right-0 z-50 bg-gradient-to-b from-neutral-950/90 via-neutral-950/50 to-transparent backdrop-blur-md">
        <div className="w-1/3 flex justify-start">
          <span className="text-sm font-bold tracking-wider text-white/90">QUÉVEOHOY</span>
        </div>
        <nav className="bg-black/50 backdrop-blur-md border border-white/10 p-1.5 rounded-full flex items-center gap-1.5 shadow-xl shadow-black/50">
          <NavButton 
            active={activeTab === "hoy"} 
            onClick={() => setActiveTab("hoy")} 
            icon={<Sparkles size={16} />} 
            label="Hoy" 
          />
          <NavButton 
            active={activeTab === "en_progreso"} 
            onClick={() => setActiveTab("en_progreso")} 
            icon={<PlayCircle size={16} />} 
            label="En progreso" 
          />
          <NavButton 
            active={activeTab === "biblioteca"} 
            onClick={() => setActiveTab("biblioteca")} 
            icon={<Library size={16} />} 
            label="Biblioteca" 
          />
          <NavButton 
            active={activeTab === "historial"} 
            onClick={() => setActiveTab("historial")} 
            icon={<Clock size={16} />} 
            label="Historial" 
          />
          <NavButton 
            active={activeTab === "buscar"} 
            onClick={() => setActiveTab("buscar")} 
            icon={<Search size={16} />} 
            label="Buscar" 
          />
        </nav>
        <div className="w-1/3 flex justify-end gap-2 items-center">
          {activeTab === "hoy" && (
            <div className="relative" ref={popoverRef}>
              <button
                onClick={() => setIsGenreOpen(!isGenreOpen)}
                className={`text-xs border border-white/10 rounded-full px-3 py-1.5 transition-all flex items-center gap-1.5 cursor-pointer relative z-50 ${selectedGeneros.length > 0 ? 'border-purple-500/40 text-purple-200 bg-purple-500/10' : 'bg-white/5 hover:bg-white/10 text-neutral-300'}`}
              >
                🎭 Géneros {selectedGeneros.length > 0 ? `(${selectedGeneros.length})` : ''} ▾
              </button>
              
              {isGenreOpen && (
                <div className="absolute right-0 top-full mt-2 w-72 p-3 bg-neutral-900/95 backdrop-blur-xl border border-white/10 rounded-2xl shadow-2xl shadow-black/80 z-50 flex flex-col gap-2 animate-in fade-in zoom-in-95 duration-150">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-semibold text-neutral-300 uppercase tracking-wider">Filtrar por género</span>
                    {selectedGeneros.length > 0 && (
                      <button 
                        onClick={(e) => { e.stopPropagation(); setSelectedGeneros([]); }}
                        className="text-[10px] text-purple-400 hover:text-purple-300 cursor-pointer"
                      >
                        Limpiar
                      </button>
                    )}
                  </div>
                  <div className="flex flex-wrap gap-1.5 max-h-48 overflow-y-auto pr-1">
                    {ALL_GENRES.map(g => (
                      <button
                        key={g}
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedGeneros(prev => prev.includes(g) ? prev.filter(x => x !== g) : [...prev, g]);
                        }}
                        className={selectedGeneros.includes(g) 
                          ? 'text-[11px] px-2 py-1 rounded-lg bg-purple-600/30 text-purple-200 border border-purple-500/50 font-medium cursor-pointer transition-all'
                          : 'text-[11px] px-2 py-1 rounded-lg bg-white/5 hover:bg-white/10 text-neutral-400 border border-white/5 cursor-pointer transition-colors'
                        }
                      >
                        {g}
                      </button>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
          {activeTab === "hoy" && recomendacion?.estado === "en_progreso" && (
            <button 
              onClick={async () => {
                console.log("[DEBUG] Saltar en progreso ejecutado");
                try {
                  await invoke("omitir_todos_en_progreso");
                  setRecomendacion(null);
                  await loadRecommendation();
                } catch (e) {
                  console.error(e);
                }
              }}
              disabled={loading}
              className="px-4 py-1.5 text-xs font-medium border border-white/10 rounded-full bg-white/5 hover:bg-white/10 text-neutral-300 transition-all pointer-events-auto cursor-pointer relative z-50 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Saltar en progreso
            </button>
          )}
        </div>
      </header>

      <div className="flex-1 overflow-y-auto flex flex-col relative pt-14">
        {activeTab === "hoy" && (
          <div className="flex flex-col h-full relative w-full">
            <ErrorBoundary>
              <HoyView
                recomendacion={recomendacion}
                history={history}
                onSiguiente={handleSiguiente}
                onAnterior={handleAnterior}
                onUpdateRecomendacion={handleUpdateRecomendacion}
                onShowToast={setToastMessage}
                loading={loading}
              />
            </ErrorBoundary>
          </div>
        )}

        {activeTab === "en_progreso" && (
          <EnProgresoView 
            onReanudar={(item: any) => {
              setRecomendacion(item);
              setActiveTab("hoy");
            }} 
          />
        )}
        {activeTab === "biblioteca" && (
          <BibliotecaView 
            onReanudar={(item: any) => {
              setRecomendacion(item);
              setActiveTab("hoy");
            }} 
          />
        )}
        {activeTab === "historial" && <HistorialView />}
        {activeTab === "buscar" && <div className="text-zinc-400 mt-10">Buscar (Próximamente)</div>}
      </div>
    </main>
  );
}

function NavButton({ active, onClick, icon, label }: { active: boolean, onClick: () => void, icon: React.ReactNode, label: string }) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-2 whitespace-nowrap
        ${active 
          ? "bg-white/15 text-white font-medium shadow-sm rounded-full px-4 py-1.5 text-sm transition-all" 
          : "text-neutral-400 hover:text-white rounded-full px-4 py-1.5 text-sm transition-colors"
        }
      `}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}

export default App;
