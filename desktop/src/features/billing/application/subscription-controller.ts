import type { BillingSnapshot } from '../model.ts';
import { shownPromoProgram } from '../promo-program.ts';
import { subscriptionConfirmation, type ConfirmationAction } from '../subscription-confirmation.ts';
import { subscriptionOverview } from '../subscription-overview.ts';
import type { TextStyleOptions } from '../../styles/model.ts';

export interface SubscriptionPorts {
  refresh(): Promise<unknown>;
  dismiss(): void;
  confirm(action: Exclude<ConfirmationAction, 'activateStarter'>): Promise<boolean>;
  activateStarter(request: {
    termsVersion: number;
    programVersion?: string;
    selection: string;
    activationOptions?: TextStyleOptions;
  }): Promise<boolean>;
  acceptPromoTerms(request: { termsVersion: number; selection: string }): Promise<boolean>;
  dismissPromoAccess(id: string, deferred: boolean): Promise<boolean>;
  refreshPromoAccess(id: string): Promise<boolean>;
  visible(): boolean;
  onFocus(action: () => void): () => void;
  after(milliseconds: number, action: () => void): () => void;
}

type Confirmation = ReturnType<typeof subscriptionConfirmation>;
export interface SubscriptionView {
  value: BillingSnapshot | null;
  selectedOffer: string;
  promoEntry: boolean;
  code: string;
  confirmation: Confirmation | null;
  purchaseConfirmed: boolean;
}

/** Owns one subscription screen lifetime. All effects enter through its ports. */
export function subscriptionController(
  ports: SubscriptionPorts,
  publish: (view: SubscriptionView) => void,
) {
  let view: SubscriptionView = {
    value: null,
    selectedOffer: '',
    promoEntry: false,
    code: '',
    confirmation: null,
    purchaseConfirmed: false,
  };
  let busy = false,
    failed = false,
    mounted = false,
    disposed = false;
  let sawPending = false,
    starterSubmitted = false;
  let refreshedScope: string | null = null;
  let refreshRequest: object | null = null;
  let consentRequest: object | null = null;
  let cancelTimer = () => {},
    removeFocus = () => {};

  function change(patch: Partial<SubscriptionView>) {
    view = { ...view, ...patch };
    publish(view);
  }

  function available() {
    return !disposed && !failed && !busy && !!view.value?.signedIn;
  }

  async function refresh() {
    if (!mounted || !available() || refreshRequest || !ports.visible()) return;
    const request = {};
    refreshRequest = request;
    refreshedScope = view.value!.scope;
    try {
      await ports.refresh();
    } finally {
      if (refreshRequest === request) refreshRequest = null;
    }
  }

  function schedule() {
    const seconds = Math.max(60, Math.min(3600, view.value?.refreshAfterSeconds ?? 300));
    cancelTimer = ports.after(seconds * 1000, () => {
      if (!mounted) return;
      void refresh();
      schedule();
    });
  }

  function update(value: BillingSnapshot | null, isBusy: boolean, hasFailed: boolean) {
    if (disposed) return;
    busy = isBusy;
    failed = hasFailed;
    if (value?.scope !== view.value?.scope || value?.signedIn !== view.value?.signedIn) {
      refreshedScope = null;
      refreshRequest = null;
      sawPending = false;
      starterSubmitted = false;
      view = {
        ...view,
        selectedOffer: '',
        promoEntry: false,
        code: '',
        confirmation: null,
        purchaseConfirmed: false,
      };
    }
    if (value && failed)
      value = {
        ...value,
        canPreviewCheckout: false,
        checkout: null,
        canActivateStarter: false,
      };
    view = { ...view, value };
    if (!value?.plans.some((item) => item.id === view.selectedOffer))
      view.selectedOffer = value?.plans.find((item) => item.current)?.id ?? value?.plans[0]?.id ?? '';
    if (value?.promo) {
      view.promoEntry = false;
      view.code = '';
    }
    if (value?.status.pendingCheckout) sawPending = true;
    if (sawPending && value && subscriptionOverview(value).confirmedPurchase) {
      view.purchaseConfirmed = true;
      sawPending = false;
    }
    if (value?.status.fresh && value.status.state !== 'active') view.purchaseConfirmed = false;
    const confirmation = view.confirmation;
    if (
      confirmation &&
      (failed ||
        !value?.signedIn ||
        confirmation.scope !== value.scope ||
        (confirmation.action === 'activateStarter' &&
          (confirmation.termsVersion !== value.termsVersion ||
            confirmation.programVersion !== shownPromoProgram(value).programVersion ||
            (starterSubmitted && !busy && value.hasAccess && !value.canActivateStarter))))
    )
      view.confirmation = null;
    publish(view);
    if (refreshedScope !== value?.scope) void refresh();
  }

  function requestConfirmation(action: ConfirmationAction) {
    if (!available()) return;
    starterSubmitted = false;
    change({ confirmation: subscriptionConfirmation(view.value!, action) });
  }

  async function confirm() {
    const confirmation = view.confirmation;
    if (!available() || !confirmation || confirmation.action === 'activateStarter') return;
    if (await ports.confirm(confirmation.action)) {
      if (!disposed && view.confirmation === confirmation) change({ confirmation: null });
    }
  }

  function activateStarter(selection: string, activationOptions?: TextStyleOptions) {
    const confirmation = view.confirmation;
    if (!available() || confirmation?.action !== 'activateStarter') return;
    starterSubmitted = true;
    return ports.activateStarter({
      termsVersion: confirmation.termsVersion,
      programVersion: confirmation.programVersion,
      selection,
      activationOptions,
    });
  }

  async function acceptPromoTerms(selection: string) {
    const access = view.value?.promoAccess;
    if (
      !available() ||
      consentRequest ||
      access?.kind !== 'acceptTerms' ||
      access.termsVersion === null
    )
      return;
    const request = {};
    const scope = view.value!.scope;
    consentRequest = request;
    try {
      const accepted = await ports.acceptPromoTerms({
        termsVersion: access.termsVersion,
        selection,
      });
      if (
        accepted &&
        !disposed &&
        view.value?.scope === scope &&
        view.value.promoAccess?.id === access.id
      )
        await ports.dismissPromoAccess(access.id, false);
    } finally {
      if (consentRequest === request) consentRequest = null;
    }
  }

  return {
    get view() {
      return view;
    },
    update,
    start() {
      if (mounted || disposed) return;
      mounted = true;
      removeFocus = ports.onFocus(() => {
        void refresh();
      });
      schedule();
      void refresh();
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      mounted = false;
      cancelTimer();
      removeFocus();
      ports.dismiss();
    },
    acceptPromoTerms,
    dismissPromoAccess() {
      const access = view.value?.promoAccess;
      if (available() && !consentRequest && access)
        return ports.dismissPromoAccess(access.id, true);
    },
    refreshPromoAccess() {
      const access = view.value?.promoAccess;
      if (available() && access?.kind === 'refresh') return ports.refreshPromoAccess(access.id);
    },
    requestConfirmation,
    closeConfirmation() {
      if (!busy) change({ confirmation: null });
    },
    confirm,
    activateStarter,
    activateWithAcceptedTerms(selection: string) {
      if (!available()) return;
      const value = view.value!;
      return ports.activateStarter({
        termsVersion: value.termsVersion,
        ...shownPromoProgram(value),
        selection,
      });
    },
    selectOffer(selectedOffer: string) {
      change({ selectedOffer });
    },
    setPromoEntry(promoEntry: boolean) {
      change({ promoEntry });
    },
    setCode(code: string) {
      change({ code });
    },
  };
}
