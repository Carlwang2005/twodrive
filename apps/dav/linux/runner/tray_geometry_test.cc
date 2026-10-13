#include "tray_geometry.h"
#include <cassert>

int main() {
  twodrive::TrayAnchor anchor;
  anchor.capture(1350, 875, {0, 0, 1440, 843});
  const auto first = anchor.panel();
  assert(first.x == 980 && first.y == 265);
  assert(first.width == 390 && first.height == 570);
  anchor.capture(1360, 870, {0, 0, 1440, 843});
  assert(anchor.panel().x == first.x && anchor.panel().y == first.y);
  // No pointer or main-window position is consulted between capture and show.
  anchor.capture(20, 15, {0, 30, 1440, 870});
  auto top = anchor.panel();
  assert(top.x == 8 && top.y == 38);
  anchor.capture(-10, 1080, {-1920, 0, 1920, 1040});
  auto second = anchor.panel();
  assert(second.x == -398 && second.y == 462);
  // Workarea changes invalidate the cached icon position, even for nearby clicks.
  anchor.capture(1350, 875, {0, 0, 1440, 843});
  anchor.capture(1355, 875, {0, 0, 1440, 800});
  assert(anchor.x == 1355 && anchor.panel().y == 222);
  anchor.capture(120, 150, {0, 0, 320, 480});
  auto small = anchor.panel();
  assert(small.x == 8 && small.y == 8);
  assert(small.width == 304 && small.height == 464);
}
