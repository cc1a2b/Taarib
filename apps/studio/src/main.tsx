// The faces first, then the token layer, then the base styles, then every
// component stylesheet — ES module imports evaluate in source order, so this is
// the order the cascade gets. The `@font-face` rules lead because a family has
// to be declared before the stack in `tokens.css` names it; a stack whose first
// name resolves to nothing is how this interface spent twenty phases rendering
// Arabic in whatever the operating system happened to offer.
import '@/khutut/khutut.css';
import '@/nizam/tokens.css';
import '@/nizam/qaida.css';

import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { istifsar } from '@/hayat/istifsar';
import { nadi } from '@/hayat/jisr';
import { binniMuwajjih } from '@/masar';
import { HajizKhata } from '@/mukawwinat/hajiz_khata';

const jidhr = document.getElementById('jidhr');

if (jidhr === null) {
  throw new Error('index.html is missing the #jidhr mount point');
}

/** How long the first paint waits on the session before going ahead without it. */
const MUHLAT_JALSA = 4000;

// The route table is built after the session is known, because an owner
// session's table contains the review console and a contributor session's
// table does not. A backend that cannot answer is treated as a contributor
// session: the narrower table is the safe one.
//
// Bounded, because this await sits between the process starting and anything
// being rendered: `#jidhr` is empty until it resolves, so a call that never
// settles is an empty window with no text, no error and nothing to report.
// The backend already caps its own keychain probe, but that cap does not cover
// the IPC channel itself failing to come up. Four seconds is longer than the
// backend's own three, so a slow-but-working keychain still wins the race and
// an owner does not lose the review console to an impatient timer.
const jalsa = await Promise.race([
  nadi('jalsati').catch(() => null),
  new Promise<null>((hall) => {
    setTimeout(() => hall(null), MUHLAT_JALSA);
  }),
]);

// The boundary sits outside the router, not inside a route: a throw during a
// route's own render would otherwise unmount the router with it and leave the
// window blank. Arabic is the fallback language here because settings have not
// been read yet at this point — the screen that reads them is what threw.
createRoot(jidhr).render(
  <StrictMode>
    <HajizKhata lugha="arabi">
      <QueryClientProvider client={istifsar}>
        <RouterProvider router={binniMuwajjih(jalsa?.malik === true)} />
      </QueryClientProvider>
    </HajizKhata>
  </StrictMode>,
);
