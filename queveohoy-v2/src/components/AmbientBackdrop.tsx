import React, { useState, useEffect, useRef } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";

export interface AmbientBackdropProps {
  backdropUrl?: string | null;
  posterUrl?: string | null;
}

interface ImageSlot {
  url: string;
  isBackdrop: boolean;
}

const resolveUrl = (url?: string | null): string | null => {
  if (!url || typeof url !== "string" || !url.trim()) return null;
  const clean = url.trim();
  if (
    clean.startsWith("http://") ||
    clean.startsWith("https://") ||
    clean.startsWith("asset://") ||
    clean.startsWith("blob:")
  ) {
    return clean;
  }
  return convertFileSrc(clean);
};

export const AmbientBackdrop: React.FC<AmbientBackdropProps> = ({
  backdropUrl,
  posterUrl,
}) => {
  const resolvedBackdrop = resolveUrl(backdropUrl);
  const resolvedPoster = resolveUrl(posterUrl);

  const initialUrl = resolvedBackdrop || resolvedPoster;
  const initialIsBackdrop = Boolean(resolvedBackdrop);

  const [slots, setSlots] = useState<{
    active: 0 | 1;
    images: [ImageSlot | null, ImageSlot | null];
  }>({
    active: 0,
    images: [initialUrl ? { url: initialUrl, isBackdrop: initialIsBackdrop } : null, null],
  });

  const lastTargetRef = useRef<string | null>(initialUrl);

  useEffect(() => {
    const primaryUrl = resolvedBackdrop;
    const fallbackUrl = resolvedPoster;
    const targetUrl = primaryUrl || fallbackUrl;
    const isBackdropCandidate = Boolean(primaryUrl);

    if (targetUrl === lastTargetRef.current) return;
    lastTargetRef.current = targetUrl;

    if (!targetUrl) {
      setSlots(prev => ({
        ...prev,
        active: prev.active === 0 ? 1 : 0,
        images: prev.active === 0 ? [prev.images[0], null] : [null, prev.images[1]],
      }));
      return;
    }

    let isCancelled = false;

    const commitSwap = (finalUrl: string, isBackdropMode: boolean) => {
      if (isCancelled) return;
      setSlots(prev => {
        const nextActive = prev.active === 0 ? 1 : 0;
        const nextImages: [ImageSlot | null, ImageSlot | null] = [...prev.images];
        nextImages[nextActive] = { url: finalUrl, isBackdrop: isBackdropMode };
        return {
          active: nextActive,
          images: nextImages,
        };
      });
    };

    const tryLoad = (urlToLoad: string, isBackdropChoice: boolean) => {
      const img = new Image();
      img.src = urlToLoad;

      img.onload = () => {
        if (!isCancelled) {
          commitSwap(urlToLoad, isBackdropChoice);
        }
      };

      img.onerror = () => {
        if (isCancelled) return;
        // Fallback automático al póster si el backdrop no pudo cargarse
        if (isBackdropChoice && fallbackUrl && fallbackUrl !== urlToLoad) {
          tryLoad(fallbackUrl, false);
        }
      };

      if (img.complete && img.naturalWidth > 0) {
        commitSwap(urlToLoad, isBackdropChoice);
      }
    };

    tryLoad(targetUrl, isBackdropCandidate);

    return () => {
      isCancelled = true;
    };
  }, [resolvedBackdrop, resolvedPoster]);

  const img0 = slots.images[0];
  const img1 = slots.images[1];
  const isActive0 = slots.active === 0 && Boolean(img0);
  const isActive1 = slots.active === 1 && Boolean(img1);

  return (
    <div className="absolute inset-0 z-0 overflow-hidden pointer-events-none select-none bg-[#080910]">
      {/* CAPA B — Glow ambiental difuminado (Slot 0) */}
      {img0 && (
        <img
          src={img0.url}
          alt=""
          className={`absolute inset-0 w-full h-full object-cover scale-125 blur-3xl transition-opacity duration-700 ease-out pointer-events-none ${
            isActive0 ? "opacity-25" : "opacity-0"
          }`}
        />
      )}

      {/* CAPA B — Glow ambiental difuminado (Slot 1) */}
      {img1 && (
        <img
          src={img1.url}
          alt=""
          className={`absolute inset-0 w-full h-full object-cover scale-125 blur-3xl transition-opacity duration-700 ease-out pointer-events-none ${
            isActive1 ? "opacity-25" : "opacity-0"
          }`}
        />
      )}

      {/* CAPA A — Imagen principal nítida, reconocible y sin blur (Slot 0) */}
      {img0 && (
        <img
          src={img0.url}
          alt=""
          className={`absolute inset-0 w-full h-full object-cover scale-[1.03] blur-none transition-opacity duration-700 ease-out pointer-events-none ${
            isActive0 ? "opacity-100" : "opacity-0"
          }`}
        />
      )}

      {/* CAPA A — Imagen principal nítida, reconocible y sin blur (Slot 1) */}
      {img1 && (
        <img
          src={img1.url}
          alt=""
          className={`absolute inset-0 w-full h-full object-cover scale-[1.03] blur-none transition-opacity duration-700 ease-out pointer-events-none ${
            isActive1 ? "opacity-100" : "opacity-0"
          }`}
        />
      )}

      {/* 3. Overlay oscuro ultra sutil (5% para unificar paleta sin apagar luminancia) */}
      <div className="absolute inset-0 bg-[#080910]/5 pointer-events-none" />

      {/* 4. Gradiente inferior localizado en el 50% inferior (protege ficha técnica y acciones sin oscurecer personajes ni escenarios) */}
      <div className="absolute bottom-0 inset-x-0 h-1/2 bg-gradient-to-t from-[#080910]/95 via-[#080910]/35 to-transparent pointer-events-none" />

      {/* 5. Viñeta radial moderada (centro 65% completamente transparente, bordes suaves a 35%) */}
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,transparent_65%,rgba(8,9,16,0.35)_100%)] pointer-events-none" />
    </div>
  );
};

export default AmbientBackdrop;
