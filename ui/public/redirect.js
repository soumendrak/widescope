// Legacy share links pointed at the site root (/?trace=…, /#trace=…, ?embed=1).
// The viewer now lives at /editor/ — forward any trace-bearing URL there intact.
(function () {
  var s = location.search, h = location.hash;
  if (/[?&](trace|view|span|embed|sample)=/.test(s) || /[#&](trace|view|span)=/.test(h)) {
    location.replace('/editor/' + s + h);
  }
})();
