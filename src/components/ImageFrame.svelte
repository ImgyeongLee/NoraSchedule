<script lang="ts">
  // Shows a stored image covering its box, positioned by a Framing (see lib/framing.ts).
  // With `editable`, dragging pans the image and the mouse wheel zooms it.
  import { cleanFraming, MAX_ZOOM, placeImage, type Framing } from '../lib/framing';
  import { imageUrl } from '../lib/images';

  let {
    image,
    framing = $bindable(),
    editable = false,
  }: { image: string; framing: Framing; editable?: boolean } = $props();

  let w = $state(0);
  let h = $state(0);
  let nw = $state(0);
  let nh = $state(0);

  const f = $derived(cleanFraming(framing));
  const box = $derived(nw && nh && w && h ? placeImage(nw, nh, w, h, f) : null);

  let drag: { x: number; y: number; start: Framing } | null = null;

  function down(e: PointerEvent) {
    if (!editable || e.button !== 0) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = { x: e.clientX, y: e.clientY, start: f };
  }

  function move(e: PointerEvent) {
    if (!drag || !nw || !nh) return;
    // Measure against the size at drag start so the image follows the pointer.
    const at = placeImage(nw, nh, w, h, drag.start);
    framing = cleanFraming({
      ...drag.start,
      x: at.overflowX > 0 ? drag.start.x - (e.clientX - drag.x) / at.overflowX : drag.start.x,
      y: at.overflowY > 0 ? drag.start.y - (e.clientY - drag.y) / at.overflowY : drag.start.y,
    });
  }

  function wheel(e: WheelEvent) {
    if (!editable) return;
    e.preventDefault();
    framing = cleanFraming({ ...f, zoom: Math.min(MAX_ZOOM, Math.max(1, f.zoom * (e.deltaY < 0 ? 1.08 : 1 / 1.08))) });
  }
</script>

<div
  class="frame"
  class:editable
  bind:clientWidth={w}
  bind:clientHeight={h}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={() => (drag = null)}
  onpointercancel={() => (drag = null)}
  onwheel={wheel}
  role="presentation"
>
  <img
    src={imageUrl(image, 'frame')}
    alt=""
    draggable="false"
    onload={(e) => {
      const img = e.currentTarget as HTMLImageElement;
      nw = img.naturalWidth;
      nh = img.naturalHeight;
    }}
    style:width={box ? `${box.width}px` : '100%'}
    style:height={box ? `${box.height}px` : '100%'}
    style:left={box ? `${box.left}px` : '0'}
    style:top={box ? `${box.top}px` : '0'}
    style:object-fit={box ? undefined : 'cover'}
  />
</div>

<style>
  .frame {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
  .frame.editable {
    cursor: grab;
    touch-action: none;
  }
  .frame.editable:active {
    cursor: grabbing;
  }
  img {
    position: absolute;
    max-width: none;
    user-select: none;
    pointer-events: none;
  }
</style>
