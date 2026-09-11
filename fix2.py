import codecs
import re

file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src\HoyView.tsx'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

# We need to find the exact line 800 and fix it.
# The line 800 is:
bad_code = '                className="w-full bg-[#6D28D9] hover:bg-[#7C3AED] text-white font-bold py-3 rounded-xl transition-colors shadow-lg shadow-purple-900/50 flex items-center justify-center gap-2"'
# We will replace from bad_code to the end of the file, because we will reconstruct the end properly.

proper_end = '''                className="w-full bg-[#6D28D9] hover:bg-[#7C3AED] text-white font-bold py-3 rounded-xl transition-colors shadow-lg shadow-purple-900/50 flex items-center justify-center gap-2"
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
                            className={px-4 py-2 rounded-xl text-sm font-semibold transition-colors flex-shrink-0 }
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
                              className={p-2 rounded-lg text-sm font-medium transition-colors }
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
'''

idx = content.find(bad_code)
if idx != -1:
    content = content[:idx] + proper_end
    with codecs.open(file_path, 'w', encoding='utf-8') as f:
        f.write(content)
    print("Fixed end of file!")
else:
    print("Could not find bad code")
