import React, { useRef, useEffect } from 'react';
import { ChartNote, GUITAR_STRINGS } from '../chart/chartModel';
import { ScoredEvent } from '../scoring/scoringEngine';

interface NoteHighway3DProps {
  notes: ChartNote[];
  currentSongTimeMs: number;
  scoredEvents: ScoredEvent[];
  comboMultiplier: number;
  detectedMidi: number | null;
  detectedFreq: number | null;
}

interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  color: string;
  alpha: number;
  size: number;
}

export const NoteHighway3D: React.FC<NoteHighway3DProps> = ({
  notes,
  currentSongTimeMs,
  scoredEvents,
  comboMultiplier,
  detectedMidi,
  detectedFreq,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const particlesRef = useRef<Particle[]>([]);
  const lastScoredCountRef = useRef<number>(0);

  // Highway rendering settings
  const travelTimeMs = 2000; // Time note takes from horizon to hitline

  useEffect(() => {
    // When a new hit occurs, spawn particle explosion
    if (scoredEvents.length > lastScoredCountRef.current) {
      const latest = scoredEvents[scoredEvents.length - 1];
      if (latest.judgment !== 'MISS' && canvasRef.current) {
        const rect = canvasRef.current;
        const width = rect.width;
        const height = rect.height;
        const hitY = height * 0.85;

        // Find string for this note
        const note = notes.find((n) => n.id === latest.noteId);
        const stringIndex = note ? note.stringIndex : 6;
        // String rail x coordinate at hit line
        const stringNorm = (6 - stringIndex) / 5; // 0 to 1
        const bottomWidth = width * 0.72;
        const bottomStartX = (width - bottomWidth) / 2;
        const hitX = bottomStartX + stringNorm * bottomWidth;

        const color = latest.judgment === 'PERFECT' ? '#FACC15' : latest.judgment === 'GREAT' ? '#38BDF8' : '#4ADE80';

        for (let i = 0; i < 28; i++) {
          const angle = Math.random() * Math.PI * 2;
          const speed = 2 + Math.random() * 5;
          particlesRef.current.push({
            x: hitX,
            y: hitY,
            vx: Math.cos(angle) * speed,
            vy: Math.sin(angle) * speed - 1.5,
            color,
            alpha: 1.0,
            size: 2 + Math.random() * 4,
          });
        }
      }
      lastScoredCountRef.current = scoredEvents.length;
    }
  }, [scoredEvents, notes]);

  useEffect(() => {
    let animationFrameId: number;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const render = () => {
      const width = canvas.width;
      const height = canvas.height;

      // 1. Clear background
      ctx.fillStyle = '#090D16';
      ctx.fillRect(0, 0, width, height);

      // Gradient horizon background glow
      const horizonY = height * 0.18;
      const hitY = height * 0.85;

      const bgGrad = ctx.createLinearGradient(0, 0, 0, height);
      bgGrad.addColorStop(0, '#040711');
      bgGrad.addColorStop(0.3, '#0b1329');
      bgGrad.addColorStop(0.85, '#070c18');
      bgGrad.addColorStop(1, '#02040a');
      ctx.fillStyle = bgGrad;
      ctx.fillRect(0, 0, width, height);

      // 2. Compute 3D Perspective Fretboard Trapazoid
      const topWidth = width * 0.28;
      const bottomWidth = width * 0.76;
      const topStartX = (width - topWidth) / 2;
      const bottomStartX = (width - bottomWidth) / 2;

      // Draw highway track base
      ctx.save();
      ctx.beginPath();
      ctx.moveTo(topStartX, horizonY);
      ctx.lineTo(topStartX + topWidth, horizonY);
      ctx.lineTo(bottomStartX + bottomWidth, hitY + 25);
      ctx.lineTo(bottomStartX, hitY + 25);
      ctx.closePath();

      const trackGrad = ctx.createLinearGradient(0, horizonY, 0, hitY);
      trackGrad.addColorStop(0, 'rgba(15, 23, 42, 0.4)');
      trackGrad.addColorStop(0.5, 'rgba(30, 41, 59, 0.7)');
      trackGrad.addColorStop(1, 'rgba(15, 23, 42, 0.95)');
      ctx.fillStyle = trackGrad;
      ctx.fill();

      // Highway borders
      ctx.lineWidth = 2.5;
      ctx.strokeStyle = '#38BDF8';
      ctx.stroke();
      ctx.restore();

      // 3. Perspective Frets (horizontal lines receding in distance)
      const numFrets = 12;
      ctx.lineWidth = 1;
      for (let f = 1; f <= numFrets; f++) {
        // Perspective distance exponential formula
        const p = Math.pow(f / numFrets, 2.2);
        const y = horizonY + (hitY - horizonY) * p;
        const fLeft = topStartX + (bottomStartX - topStartX) * p;
        const fRight = topStartX + topWidth + ((bottomStartX + bottomWidth) - (topStartX + topWidth)) * p;

        ctx.strokeStyle = `rgba(148, 163, 184, ${0.08 + p * 0.35})`;
        ctx.beginPath();
        ctx.moveTo(fLeft, y);
        ctx.lineTo(fRight, y);
        ctx.stroke();
      }

      // 4. Draw String Rails (6 strings with authentic Rocksmith colors)
      const stringProps = GUITAR_STRINGS; // 6=E, 5=A, 4=D, 3=G, 2=B, 1=e
      for (let s = 0; s < 6; s++) {
        const norm = s / 5; // 0 (Low E) to 1 (High E)
        const topX = topStartX + norm * topWidth;
        const botX = bottomStartX + norm * bottomWidth;
        const stringColor = stringProps[s].color;

        // String rail line
        ctx.strokeStyle = stringColor;
        ctx.lineWidth = 1.8 + norm * 0.8;
        ctx.beginPath();
        ctx.moveTo(topX, horizonY);
        ctx.lineTo(botX, hitY + 20);
        ctx.stroke();

        // Label on fretboard at bottom
        ctx.fillStyle = stringColor;
        ctx.font = 'bold 12px Inter, system-ui, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(stringProps[s].name, botX, hitY + 38);
      }

      // 5. Hit Target Bar (Laser line at hitY)
      const hitGrad = ctx.createLinearGradient(bottomStartX, hitY, bottomStartX + bottomWidth, hitY);
      hitGrad.addColorStop(0, 'rgba(239, 68, 68, 0.8)');
      hitGrad.addColorStop(0.2, 'rgba(234, 179, 8, 0.8)');
      hitGrad.addColorStop(0.4, 'rgba(59, 130, 246, 0.8)');
      hitGrad.addColorStop(0.6, 'rgba(249, 115, 22, 0.8)');
      hitGrad.addColorStop(0.8, 'rgba(16, 185, 129, 0.8)');
      hitGrad.addColorStop(1, 'rgba(168, 85, 247, 0.8)');

      ctx.save();
      ctx.shadowColor = '#38BDF8';
      ctx.shadowBlur = 12;
      ctx.lineWidth = 4;
      ctx.strokeStyle = hitGrad;
      ctx.beginPath();
      ctx.moveTo(bottomStartX - 8, hitY);
      ctx.lineTo(bottomStartX + bottomWidth + 8, hitY);
      ctx.stroke();
      ctx.restore();

      // 6. Draw Scrolling 3D Note Gems
      for (const note of notes) {
        const timeDiff = note.timestampMs - currentSongTimeMs;

        // Show notes within travel window (-200ms to +travelTimeMs)
        if (timeDiff >= -250 && timeDiff <= travelTimeTimeMsSafe(travelTimeMs)) {
          // Progress from 0 (at horizon) to 1 (at hit target)
          const rawProgress = 1 - timeDiff / travelTimeMs;
          // Apply perspective curve
          const p = Math.pow(Math.max(0, Math.min(1.05, rawProgress)), 1.85);

          const y = horizonY + (hitY - horizonY) * p;

          // String rail position
          const sIdx = 6 - note.stringIndex; // 0 to 5
          const norm = sIdx / 5;
          const leftAtP = topStartX + (bottomStartX - topStartX) * p;
          const widthAtP = topWidth + (bottomWidth - topWidth) * p;
          const x = leftAtP + norm * widthAtP;

          const baseColor = GUITAR_STRINGS[sIdx]?.color || '#38BDF8';
          const size = 12 + p * 22; // Gem expands as it approaches camera

          // Draw sustain tail if duration > 300ms
          if (note.durationMs > 300) {
            const tailTimeDiff = (note.timestampMs + note.durationMs) - currentSongTimeMs;
            const tailProgress = 1 - tailTimeDiff / travelTimeMs;
            const tailP = Math.pow(Math.max(0, Math.min(1.05, tailProgress)), 1.85);
            const tailY = horizonY + (hitY - horizonY) * tailP;
            const tailLeftAtP = topStartX + (bottomStartX - topStartX) * tailP;
            const tailWidthAtP = topWidth + (bottomWidth - topWidth) * tailP;
            const tailX = tailLeftAtP + norm * tailWidthAtP;

            ctx.save();
            ctx.beginPath();
            ctx.moveTo(x - size * 0.25, y);
            ctx.lineTo(x + size * 0.25, y);
            ctx.lineTo(tailX + size * 0.15, tailY);
            ctx.lineTo(tailX - size * 0.15, tailY);
            ctx.closePath();
            ctx.fillStyle = `${baseColor}55`;
            ctx.fill();
            ctx.restore();
          }

          // Draw Note Gem
          ctx.save();
          ctx.shadowColor = baseColor;
          ctx.shadowBlur = 8 + p * 12;

          // 3D Gem polygon (diamond/rectangle with bevel)
          ctx.fillStyle = baseColor;
          const h = size * 0.65;
          const w = size * 0.95;

          ctx.beginPath();
          ctx.roundRect(x - w / 2, y - h / 2, w, h, [4 + p * 3]);
          ctx.fill();

          // Border shine
          ctx.strokeStyle = '#FFFFFF';
          ctx.lineWidth = 1.5;
          ctx.stroke();

          // Fret Number label
          ctx.fillStyle = '#FFFFFF';
          ctx.font = `bold ${Math.max(10, Math.round(9 + p * 6))}px Inter, system-ui, sans-serif`;
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillText(note.fretNumber === 0 ? 'O' : `${note.fretNumber}`, x, y);
          ctx.restore();
        }
      }

      // 7. Render Particle Hit Bursts
      for (let i = particlesRef.current.length - 1; i >= 0; i--) {
        const pt = particlesRef.current[i];
        pt.x += pt.vx;
        pt.y += pt.vy;
        pt.vy += 0.15; // Gravity
        pt.alpha -= 0.025;

        if (pt.alpha <= 0) {
          particlesRef.current.splice(i, 1);
          continue;
        }

        ctx.save();
        ctx.globalAlpha = Math.max(0, pt.alpha);
        ctx.fillStyle = pt.color;
        ctx.beginPath();
        ctx.arc(pt.x, pt.y, pt.size, 0, Math.PI * 2);
        ctx.fill();
        ctx.restore();
      }

      // 8. Latest Scored Judgment Banner
      if (scoredEvents.length > 0) {
        const latest = scoredEvents[scoredEvents.length - 1];
        const age = currentSongTimeMs - latest.timestampMs;
        if (age < 850) {
          const fade = 1 - age / 850;
          ctx.save();
          ctx.globalAlpha = fade;
          ctx.textAlign = 'center';
          ctx.font = 'bold 28px Inter, system-ui, sans-serif';

          if (latest.judgment === 'PERFECT') {
            ctx.fillStyle = '#FACC15';
            ctx.shadowColor = '#FACC15';
            ctx.shadowBlur = 18;
            ctx.fillText('★ PERFECT ★', width / 2, hitY - 45 - (1 - fade) * 20);
          } else if (latest.judgment === 'GREAT') {
            ctx.fillStyle = '#38BDF8';
            ctx.shadowColor = '#38BDF8';
            ctx.shadowBlur = 14;
            ctx.fillText('GREAT!', width / 2, hitY - 45 - (1 - fade) * 20);
          } else if (latest.judgment === 'GOOD') {
            ctx.fillStyle = '#4ADE80';
            ctx.shadowColor = '#4ADE80';
            ctx.shadowBlur = 10;
            ctx.fillText('GOOD', width / 2, hitY - 45 - (1 - fade) * 20);
          } else {
            ctx.fillStyle = '#F87171';
            ctx.shadowColor = '#EF4444';
            ctx.shadowBlur = 8;
            ctx.fillText('MISS', width / 2, hitY - 45 - (1 - fade) * 20);
          }

          // Timing offset display
          ctx.font = '500 13px Inter, monospace';
          ctx.fillStyle = 'rgba(255, 255, 255, 0.8)';
          const sign = latest.timeDiffMs > 0 ? '+' : '';
          ctx.fillText(
            `${sign}${Math.round(latest.timeDiffMs)} ms | ${latest.expectedNote}`,
            width / 2,
            hitY - 22 - (1 - fade) * 20
          );
          ctx.restore();
        }
      }

      // 9. Streak Multiplier Glow Badge
      if (comboMultiplier > 1) {
        ctx.save();
        ctx.fillStyle = comboMultiplier === 4 ? '#A855F7' : comboMultiplier === 3 ? '#38BDF8' : '#FACC15';
        ctx.font = 'bold 15px Inter, system-ui, sans-serif';
        ctx.textAlign = 'left';
        ctx.fillText(`${comboMultiplier}X MULTIPLIER`, bottomStartX + 10, hitY - 15);
        ctx.restore();
      }

      animationFrameId = requestAnimationFrame(render);
    };

    animationFrameId = requestAnimationFrame(render);
    return () => cancelAnimationFrame(animationFrameId);
  }, [notes, currentSongTimeMs, scoredEvents, comboMultiplier, detectedMidi, travelTimeMs]);

  return (
    <div className="relative w-full h-[460px] rounded-xl overflow-hidden shadow-2xl border border-slate-800/80 bg-slate-950">
      <canvas
        ref={canvasRef}
        width={960}
        height={460}
        className="w-full h-full block"
      />
    </div>
  );
};

function travelTimeTimeMsSafe(val: number): number {
  return val + 100;
}
