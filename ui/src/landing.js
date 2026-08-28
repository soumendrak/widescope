/*
 * Landing-page behaviour. Lives in its own file, never inline: a
 * script-blocking CSP silently killed the inline version in Cloudflare
 * production, and every section was invisible because the reveal animation
 * had not run. Content is now visible by default (see `.reveal` in the page
 * CSS) and this file only *adds* motion — if it never loads, the page still
 * reads correctly.
 */
// Legacy share links pointed at the site root (/?trace=…, /#trace=…, ?embed=1).
// The viewer now lives at /editor/ — forward any trace-bearing URL there intact.
(function () {
  var s = location.search, h = location.hash;
  if (/[?&](trace|view|span|embed|sample)=/.test(s) || /[#&](trace|view|span)=/.test(h)) {
    location.replace('/editor/' + s + h);
  }
})();


/*
 * Theme. Shares the editor's storage key so crossing / -> /editor/ keeps the
 * same look; the dark-default resolution matches lib/theme.ts.
 */
(function initTheme() {
  var KEY = 'widescope:theme';
  var stored = null;
  try { stored = localStorage.getItem(KEY); } catch (e) { /* storage blocked */ }
  var theme = stored === 'light' || stored === 'dark'
    ? stored
    : (matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark');
  document.documentElement.setAttribute('data-theme', theme);

  document.addEventListener('click', function (e) {
    var btn = e.target.closest && e.target.closest('#themeToggle');
    if (!btn) return;
    var next = document.documentElement.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', next);
    try { localStorage.setItem(KEY, next); } catch (err) { /* storage blocked */ }
  });
})();

// Opt the page into JS-driven motion. Everything animated is hidden only
// under `.js-motion`, so a blocked or failed script leaves content visible.
document.documentElement.classList.add('js-motion');

(function(){
  "use strict";
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

  /* ----- header state ----- */
  const hdr = document.getElementById("hdr");
  const onScrollHdr = () => hdr.classList.toggle("scrolled", scrollY > 24);
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

  /* ----- hero tabs: flame <-> timeline (replays build animation) ----- */
  const tabs = document.querySelectorAll(".tab");
  const lyrs = document.querySelectorAll(".lyr");
  tabs.forEach(tab => tab.addEventListener("click", () => {
    tabs.forEach(t => { t.classList.toggle("on", t === tab); t.setAttribute("aria-selected", t === tab); });
    lyrs.forEach(l => {
      const on = l.dataset.lyr === tab.dataset.view;
      l.classList.toggle("off", !on);
      l.classList.remove("act");
      if(on){ void l.offsetWidth; l.classList.add("act"); } // reflow → replay cascade
    });
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

  /* ----- cursor glow on cards ----- */
  document.querySelectorAll(".card, .zero").forEach(c => {
    c.addEventListener("mousemove", e => {
      const r = c.getBoundingClientRect();
      c.style.setProperty("--mx", (e.clientX - r.left) + "px");
      c.style.setProperty("--my", (e.clientY - r.top) + "px");
    });
  });

  document.getElementById("yr").textContent = new Date().getFullYear();
})();

