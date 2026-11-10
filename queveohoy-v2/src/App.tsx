import React, { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { EnProgresoView } from "./EnProgresoView";
import { BibliotecaView } from "./BibliotecaView";
import { HistorialView } from "./HistorialView";
import { HoyView } from "./HoyView";
import { BuscarView } from "./BuscarView";
import { FiltrosConfig } from "./utils/filters";
import {
  Home,
  Calendar,
  Bookmark,
  BarChart2,
  Settings,
  Play,
  PlayCircle,
  Sparkles,
  Library,
  Clock,
  Search,
  Bell
} from "lucide-react";
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

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("hoy");
  const [recomendacion, setRecomendacion] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  const [history, setHistory] = useState<any[]>([]);
  const [forwardHistory, setForwardHistory] = useState<any[]>([]);

  // Prevent double fetch in Strict Mode
  const isFetchingRef = useRef(false);

  useEffect(() => {
    if (toastMessage) {
      const timer = setTimeout(() => setToastMessage(null), 2500);
      return () => clearTimeout(timer);
    }
  }, [toastMessage]);

  const [filtrosHoy, setFiltrosHoy] = useState<FiltrosConfig>({ tipos: [], generos: [] });
  const filtrosHoyRef = useRef<FiltrosConfig>(filtrosHoy);
  filtrosHoyRef.current = filtrosHoy;

  const loadRecommendation = async (customFiltros?: FiltrosConfig, retryCount = 0) => {
    if (isFetchingRef.current && retryCount === 0) return;
    isFetchingRef.current = true;
    if (retryCount === 0) setLoading(true);
    
    const activeFiltros = customFiltros !== undefined ? customFiltros : filtrosHoyRef.current;

    try {
      const hasFilters = (activeFiltros.tipos && activeFiltros.tipos.length > 0) || (activeFiltros.generos && activeFiltros.generos.length > 0);
      let rec = await invoke("get_recommendation", {
        filtros: hasFilters ? { tipos: activeFiltros.tipos, generos: activeFiltros.generos } : null,
        generos: activeFiltros.generos && activeFiltros.generos.length > 0 ? activeFiltros.generos : null,
      });
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
  }, [activeTab, recomendacion]);

  const handleApplyFiltros = async (newFiltros: FiltrosConfig) => {
    setFiltrosHoy(newFiltros);
    filtrosHoyRef.current = newFiltros;
    setForwardHistory([]);
    setRecomendacion(null);
    await loadRecommendation(newFiltros);
  };

  const handleClearFiltros = async () => {
    const emptyFilters: FiltrosConfig = { tipos: [], generos: [] };
    setFiltrosHoy(emptyFilters);
    filtrosHoyRef.current = emptyFilters;
    setForwardHistory([]);
    setRecomendacion(null);
    await loadRecommendation(emptyFilters);
  };

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

  const handleSaltarEnProgreso = async () => {
    try {
      await invoke("omitir_todos_en_progreso");
      setRecomendacion(null);
      await loadRecommendation();
    } catch (error) {
      console.error("Error al saltar en progreso:", error);
    }
  };

  const handleReanudar = (item: any) => {
    setLoading(false);
    setRecomendacion({ ...item });
    setActiveTab("hoy");
  };

  return (
    <main className="h-screen w-screen overflow-hidden bg-zinc-950 flex flex-col text-white font-sans select-none">
      {/* Global Toast Modal */}
      {toastMessage && (
        <div className="fixed top-18 left-1/2 transform -translate-x-1/2 z-[100] bg-zinc-900/95 border border-violet-500/50 shadow-[0_0_25px_rgba(139,92,246,0.35)] rounded-full px-6 py-2.5 flex items-center gap-3 animate-in slide-in-from-top-4 fade-in duration-300 backdrop-blur-md">
          <div className="text-xl">✨</div>
          <div className="font-semibold text-white text-xs md:text-sm">{toastMessage}</div>
        </div>
      )}

      {/* 1. HEADER CHROME SUPERIOR (INCLUYE LOGO EN LA ESQUINA SUPERIOR IZQUIERDA) */}
      <header className="h-14 px-4 flex items-center justify-between w-full fixed top-0 left-0 right-0 z-40 bg-neutral-950/80 backdrop-blur-md border-b border-white/5">
        {/* Izquierda: Logotipo y pestañas principales */}
        <div className="flex items-center gap-8">
          {/* Logo QuéVeoHoy en la esquina superior izquierda */}
          <div
            className="flex items-center gap-2.5 cursor-pointer select-none"
            onClick={() => setActiveTab("hoy")}
          >
            <div className="w-7 h-7 rounded-lg bg-gradient-to-tr from-violet-600 to-indigo-600 flex items-center justify-center text-white shadow-md shadow-violet-900/50">
              <Play size={13} fill="currentColor" className="ml-0.5" />
            </div>
            <span className="font-bold text-white tracking-tight text-base">QuéVeoHoy</span>
          </div>

          {/* Pestañas de Navegación Horizontales */}
          <nav className="flex items-center gap-6">
            <button
              onClick={() => setActiveTab("hoy")}
              className={`flex items-center gap-1.5 text-xs md:text-sm transition-all cursor-pointer ${
                activeTab === "hoy"
                  ? "border-b-2 border-violet-500 pb-1 font-medium text-white shadow-[0_2px_10px_rgba(139,92,246,0.6)] -mb-[1px]"
                  : "text-zinc-400 hover:text-zinc-200 pb-1 transition-colors -mb-[1px]"
              }`}
            >
              <Sparkles size={15} />
              <span>Hoy</span>
            </button>
            <button
              onClick={() => setActiveTab("en_progreso")}
              className={`flex items-center gap-1.5 text-xs md:text-sm transition-all cursor-pointer ${
                activeTab === "en_progreso"
                  ? "border-b-2 border-violet-500 pb-1 font-medium text-white shadow-[0_2px_10px_rgba(139,92,246,0.6)] -mb-[1px]"
                  : "text-zinc-400 hover:text-zinc-200 pb-1 transition-colors -mb-[1px]"
              }`}
            >
              <PlayCircle size={15} />
              <span>En progreso</span>
            </button>
            <button
              onClick={() => setActiveTab("biblioteca")}
              className={`flex items-center gap-1.5 text-xs md:text-sm transition-all cursor-pointer ${
                activeTab === "biblioteca"
                  ? "border-b-2 border-violet-500 pb-1 font-medium text-white shadow-[0_2px_10px_rgba(139,92,246,0.6)] -mb-[1px]"
                  : "text-zinc-400 hover:text-zinc-200 pb-1 transition-colors -mb-[1px]"
              }`}
            >
              <Library size={15} />
              <span>Biblioteca</span>
            </button>
            <button
              onClick={() => setActiveTab("historial")}
              className={`flex items-center gap-1.5 text-xs md:text-sm transition-all cursor-pointer ${
                activeTab === "historial"
                  ? "border-b-2 border-violet-500 pb-1 font-medium text-white shadow-[0_2px_10px_rgba(139,92,246,0.6)] -mb-[1px]"
                  : "text-zinc-400 hover:text-zinc-200 pb-1 transition-colors -mb-[1px]"
              }`}
            >
              <Clock size={15} />
              <span>Historial</span>
            </button>
          </nav>
        </div>

        {/* Derecha: Búsqueda, Notificaciones y Avatar (sin botones duplicados de ventana) */}
        <div className="flex items-center gap-3">
          {/* Botón de Búsqueda */}
          <button
            onClick={() => setActiveTab("buscar")}
            className={`p-1.5 rounded-lg transition-colors cursor-pointer ${
              activeTab === "buscar"
                ? "text-violet-400 bg-violet-600/10"
                : "text-zinc-400 hover:text-white hover:bg-white/5"
            }`}
            title="Buscar"
          >
            <Search size={16} />
          </button>

          {/* Botón de Notificaciones */}
          <button
            onClick={() => setToastMessage("No hay nuevas notificaciones")}
            className="p-1.5 text-zinc-400 hover:text-white rounded-lg hover:bg-white/5 transition-colors cursor-pointer"
            title="Notificaciones"
          >
            <Bell size={16} />
          </button>

          {/* Avatar de Usuario */}
          <div
            className="w-7 h-7 rounded-full bg-gradient-to-tr from-violet-600 to-indigo-500 border border-white/20 flex items-center justify-center text-xs font-semibold text-white shadow overflow-hidden cursor-pointer ml-1"
            title="Perfil de Usuario"
          >
            <span>A</span>
          </div>
        </div>
      </header>

      {/* 2. BARRA LATERAL IZQUIERDA CONTINUA (ARRANCA JUSTO DEBAJO DEL HEADER) */}
      <aside className="w-14 bg-neutral-950/70 border-r border-white/5 flex flex-col items-center py-4 justify-between z-30 fixed left-0 top-14 bottom-0 backdrop-blur-md">
        {/* Bloque superior de accesos */}
        <div className="flex flex-col items-center gap-3">
          <button
            onClick={() => setActiveTab("hoy")}
            className={`w-9 h-9 rounded-xl flex items-center justify-center transition-all cursor-pointer ${
              activeTab === "hoy"
                ? "bg-violet-600 text-white shadow-md shadow-violet-900/40"
                : "text-zinc-400 hover:text-white hover:bg-white/5"
            }`}
            title="Inicio"
          >
            <Home size={17} />
          </button>
          <button
            onClick={() => setToastMessage("Próximos estrenos")}
            className="w-9 h-9 rounded-xl flex items-center justify-center text-zinc-400 hover:text-white hover:bg-white/5 transition-all cursor-pointer"
            title="Calendario"
          >
            <Calendar size={17} />
          </button>
          <button
            onClick={() => setActiveTab("biblioteca")}
            className={`w-9 h-9 rounded-xl flex items-center justify-center transition-all cursor-pointer ${
              activeTab === "biblioteca"
                ? "bg-violet-600 text-white shadow-md shadow-violet-900/40"
                : "text-zinc-400 hover:text-white hover:bg-white/5"
            }`}
            title="Guardados / Biblioteca"
          >
            <Bookmark size={17} />
          </button>
          <button
            onClick={() => setActiveTab("historial")}
            className={`w-9 h-9 rounded-xl flex items-center justify-center transition-all cursor-pointer ${
              activeTab === "historial"
                ? "bg-violet-600 text-white shadow-md shadow-violet-900/40"
                : "text-zinc-400 hover:text-white hover:bg-white/5"
            }`}
            title="Historial y estadísticas"
          >
            <BarChart2 size={17} />
          </button>
        </div>

        {/* Bloque inferior: Ajustes */}
        <div className="flex flex-col items-center">
          <button
            onClick={() => setToastMessage("Configuración")}
            className="w-9 h-9 rounded-xl flex items-center justify-center text-zinc-400 hover:text-white hover:bg-white/5 transition-all cursor-pointer"
            title="Ajustes"
          >
            <Settings size={17} />
          </button>
        </div>
      </aside>

      {/* ÁREA DE CONTENIDO PRINCIPAL */}
      <div className="flex-1 overflow-y-auto flex flex-col relative pl-14 pt-14 h-screen">
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
                onSaltarEnProgreso={handleSaltarEnProgreso}
                loading={loading}
                filtrosHoy={filtrosHoy}
                onApplyFiltros={handleApplyFiltros}
                onClearFiltros={handleClearFiltros}
              />
            </ErrorBoundary>
          </div>
        )}

        {activeTab === "en_progreso" && (
          <EnProgresoView onReanudar={handleReanudar} />
        )}
        {activeTab === "biblioteca" && (
          <BibliotecaView onReanudar={handleReanudar} />
        )}
        {activeTab === "historial" && <HistorialView />}
        {activeTab === "buscar" && (
          <BuscarView
            onShowToast={setToastMessage}
            onReanudar={handleReanudar}
          />
        )}
      </div>
    </main>
  );
}

export default App;
