(function(){
  "use strict";
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

  /* ----- header state + scroll-to-top floater ----- */
  const hdr = document.getElementById("hdr");
  const toTop = document.getElementById("toTop");
  const onScrollHdr = () => {
    hdr.classList.toggle("scrolled", scrollY > 24);
    toTop.classList.toggle("show", scrollY > innerHeight * 0.6);
  };
  addEventListener("scroll", onScrollHdr, {passive:true}); onScrollHdr();

  /* ----- scroll ruler: the page is a 2,400 ms trace ----- */
  const fill = document.getElementById("rulerFill");
  const ms = document.getElementById("rulerMs");
  const TRACE_MS = 2400;
  const onScrollRuler = () => {
    const max = document.documentElement.scrollHeight - innerHeight;
    const p = max > 0 ? Math.min(1, scrollY / max) : 0;
    fill.style.width = (p * 100).toFixed(2) + "%";
    ms.textContent = Math.round(p * TRACE_MS).toLocaleString() + " ms";
  };
  addEventListener("scroll", onScrollRuler, {passive:true}); onScrollRuler();

  /* ----- reveal on scroll ----- */
  const io = new IntersectionObserver(es => es.forEach(e => {
    if(e.isIntersecting){ e.target.classList.add("in"); io.unobserve(e.target); }
  }), {threshold:.18, rootMargin:"0px 0px -40px 0px"});
  document.querySelectorAll(".reveal").forEach(el => io.observe(el));

  /* ----- animated counters ----- */
  const fmt = n => Math.round(n).toLocaleString();
  const cio = new IntersectionObserver(es => es.forEach(e => {
    if(!e.isIntersecting) return;
    const el = e.target, target = +el.dataset.count;
    cio.unobserve(el);
    if(reduced){ el.textContent = fmt(target); return; }
    const t0 = performance.now(), dur = 1300;
    (function tick(t){
      const p = Math.min(1, (t - t0) / dur), ease = 1 - Math.pow(1 - p, 3);
      el.textContent = fmt(target * ease);
      if(p < 1) requestAnimationFrame(tick);
    })(t0);
  }), {threshold:.6});
  document.querySelectorAll("[data-count]").forEach(el => cio.observe(el));

  /* ----- inspector token bars ----- */
  const insp = document.getElementById("inspector");
  if(insp){
    const iio = new IntersectionObserver(es => es.forEach(e => {
      if(e.isIntersecting){ insp.classList.add("in-view"); iio.disconnect(); }
    }), {threshold:.4});
    iio.observe(insp);
  }

  /* ----- hero tabs: flame / timeline / waterfall / graph -----
     pinned scope advances views with scroll; clicking a tab scrolls to it. */
  const tabs = document.querySelectorAll(".tab");
  const lyrs = document.querySelectorAll(".lyr");
  const pin = document.querySelector(".pinwrap");
  const views = Array.from(tabs).map(t => t.dataset.view);
  let current = null;
  function setView(view){
    if(view === current) return;
    current = view;
    tabs.forEach(t => { const on = t.dataset.view === view; t.classList.toggle("on", on); t.setAttribute("aria-selected", on); });
    lyrs.forEach(l => {
      const on = l.dataset.lyr === view;
      l.classList.toggle("off", !on);
      l.classList.remove("act");
      if(on){ void l.offsetWidth; l.classList.add("act"); } // reflow → replay cascade
    });
  }
  setView("flame");

  // pinning only kicks in on wider, motion-OK viewports (matches CSS)
  const pinned = () => pin && pin.offsetHeight > innerHeight * 1.5 && !reduced;

  let viewRAF = null;
  const onScrollViews = () => {
    if(viewRAF || !pinned()) return;
    viewRAF = requestAnimationFrame(() => {
      viewRAF = null;
      const span = pin.offsetHeight - innerHeight;       // scrollable distance while pinned
      const p = Math.min(1, Math.max(0, -pin.getBoundingClientRect().top / span));
      setView(views[Math.min(views.length - 1, Math.floor(p * views.length))]);
    });
  };
  addEventListener("scroll", onScrollViews, {passive:true}); onScrollViews();

  tabs.forEach((tab, i) => tab.addEventListener("click", () => {
    if(pinned()){
      const span = pin.offsetHeight - innerHeight;
      scrollTo({ top: pin.offsetTop + (i + 0.5) / views.length * span, behavior: reduced ? "auto" : "smooth" });
    } else {
      setView(tab.dataset.view);
    }
  }));

  /* ----- span tooltip ----- */
  const tip = document.getElementById("tip");
  let tipRAF = null;
  document.querySelectorAll(".tspan").forEach(s => {
    s.addEventListener("mouseenter", () => {
      tip.innerHTML = '<div class="t-name">' + s.dataset.n + '</div>' +
        '<div class="t-ms">' + s.dataset.ms + '</div>' +
        (s.dataset.x ? '<div class="t-x">' + s.dataset.x + '</div>' : '');
      tip.classList.add("on");
    });
    s.addEventListener("mousemove", e => {
      if(tipRAF) return;
      tipRAF = requestAnimationFrame(() => {
        tipRAF = null;
        const w = tip.offsetWidth, vw = innerWidth;
        let x = e.clientX + 16; if(x + w > vw - 12) x = e.clientX - w - 16;
        tip.style.left = x + "px";
        tip.style.top = Math.max(10, e.clientY - tip.offsetHeight - 14) + "px";
      });
    });
    s.addEventListener("mouseleave", () => tip.classList.remove("on"));
  });

  /* ----- diff A/B switch with auto-cycle ----- */
  const board = document.getElementById("diffboard");
  const abBtns = document.querySelectorAll(".abswitch button");
  const candLabel = document.getElementById("candLabel");
  const totalMs = document.getElementById("totalMs");
  const totalDelta = document.getElementById("totalDelta");
  let userTouched = false, cycler = null;
  function setRun(run){
    board.classList.toggle("b", run === "b");
    abBtns.forEach(b => b.classList.toggle("on", b.dataset.run === run));
    if(run === "b"){
      candLabel.textContent = "v1.3.0-rc";
      totalMs.textContent = "2,407 ms";
      totalDelta.textContent = "−1.0%"; totalDelta.style.color = "var(--green)";
    } else {
      candLabel.textContent = "v1.2.0";
      totalMs.textContent = "2,431 ms";
      totalDelta.textContent = "±0.0%"; totalDelta.style.color = "var(--faint)";
    }
  }
  abBtns.forEach(b => b.addEventListener("click", () => {
    userTouched = true; clearInterval(cycler); setRun(b.dataset.run);
  }));
  const dio = new IntersectionObserver(es => es.forEach(e => {
    if(!e.isIntersecting) return;
    dio.disconnect();
    if(reduced){ setRun("b"); return; }
    setTimeout(() => { if(!userTouched) setRun("b"); }, 1600);
    cycler = setInterval(() => { if(!userTouched) setRun(board.classList.contains("b") ? "a" : "b"); }, 4600);
  }), {threshold:.45});
  dio.observe(board);

  /* ----- magnetic primary CTAs (desktop pointers only) ----- */
  if(matchMedia("(pointer:fine)").matches && !reduced){
    document.querySelectorAll(".magnet").forEach(btn => {
      btn.addEventListener("mousemove", e => {
        const r = btn.getBoundingClientRect();
        const dx = (e.clientX - r.left - r.width / 2) / r.width;
        const dy = (e.clientY - r.top - r.height / 2) / r.height;
        btn.style.transform = "translate(" + (dx * 7) + "px," + (dy * 5 - 2) + "px)";
      });
      btn.addEventListener("mouseleave", () => { btn.style.transform = ""; });
    });
  }

  /* ----- cursor glow + 3D tilt on cards ----- */
  const tilt3d = matchMedia("(pointer:fine)").matches && !reduced;
  document.querySelectorAll(".card, .zero").forEach(c => {
    c.addEventListener("mousemove", e => {
      const r = c.getBoundingClientRect();
      const x = e.clientX - r.left, y = e.clientY - r.top;
      c.style.setProperty("--mx", x + "px");
      c.style.setProperty("--my", y + "px");
      if(tilt3d){
        c.style.setProperty("--cry", ((x / r.width - .5) * 7).toFixed(2) + "deg");
        c.style.setProperty("--crx", ((.5 - y / r.height) * 6).toFixed(2) + "deg");
      }
    });
    c.addEventListener("mouseleave", () => {
      c.style.setProperty("--crx", "0deg");
      c.style.setProperty("--cry", "0deg");
    });
  });

  /* ----- 3D tilt on the hero scope: flattens as it centers, steers with the pointer ----- */
  const scope3d = document.querySelector(".scope");
  if(scope3d && tilt3d){
    let px = 0, py = 0, raf3d = null;
    const apply3d = () => {
      raf3d = null;
      const r = scope3d.getBoundingClientRect();
      const c = (r.top + r.height / 2 - innerHeight / 2) / innerHeight; // 0 = vertically centered
      const sx = Math.max(-4, Math.min(12, c * 16));
      scope3d.style.setProperty("--rx", (sx + py).toFixed(2) + "deg");
      scope3d.style.setProperty("--ry", px.toFixed(2) + "deg");
    };
    const queue3d = () => { if(!raf3d) raf3d = requestAnimationFrame(apply3d); };
    addEventListener("scroll", queue3d, {passive:true});
    scope3d.addEventListener("mousemove", e => {
      const r = scope3d.getBoundingClientRect();
      px = ((e.clientX - r.left) / r.width - .5) * 6;
      py = (.5 - (e.clientY - r.top) / r.height) * 5;
      queue3d();
    });
    scope3d.addEventListener("mouseleave", () => { px = 0; py = 0; queue3d(); });
    apply3d();
  }

  document.getElementById("yr").textContent = new Date().getFullYear();
})();
