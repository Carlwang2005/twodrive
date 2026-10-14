#include "tray_geometry.h"
#include <cassert>

int main() {
  twodrive::TrayAnchor anchor;
  anchor.capture_icon({850, 850, 20, 20}, {0, 0, 1440, 843});
  auto first = anchor.panel();
  assert(first.x + first.width / 2 == 860);
  assert(first.y + first.height == 835);
  assert(first.width == 390 && first.height == 570);
  // Exact icon geometry wins over a stale XWayland cursor in the workarea.
  anchor.capture(1200, 150, {0, 0, 1440, 843});
  anchor.capture_icon({850, 850, 20, 20}, {0, 0, 1440, 843});
  assert(anchor.panel().x == first.x && anchor.panel().y == first.y);
  // Host layout changes recapture the actual icon even inside 28px.
  anchor.capture_icon({860, 850, 20, 20}, {0, 0, 1440, 843});
  assert(anchor.panel().x == first.x + 10);
  anchor.capture_icon({710, 5, 20, 20}, {0, 30, 1440, 870});
  auto top = anchor.panel();
  assert(top.x == 525 && top.y == 38);
  anchor.capture_icon({-700, 1050, 20, 20}, {-1920, 0, 1920, 1040});
  auto second = anchor.panel();
  assert(second.x == -885 && second.y == 462);
  // Only screen edges may constrain horizontal centering.
  anchor.capture_icon({10, 850, 20, 20}, {0, 0, 1440, 843});
  assert(anchor.panel().x == 8);
  anchor.capture_icon({1410, 850, 20, 20}, {0, 0, 1440, 843});
  assert(anchor.panel().x == 1042);
  anchor.capture_icon({120, 485, 20, 20}, {0, 0, 320, 480});
  auto small = anchor.panel();
  assert(small.x == 8 && small.y == 8);
  assert(small.width == 304 && small.height == 464);
  // Point-only hosts still align with the taskbar edge and center horizontally.
  anchor.capture(860, 867, {0, 0, 1440, 843});
  assert(anchor.panel().x == first.x && anchor.panel().y == first.y);
}
