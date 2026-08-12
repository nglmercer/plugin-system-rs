import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import QRCode from 'qrcode';
import { fetchLocalIp } from '../lib/api';
import { t } from '../lib/i18n';
import { Modal, ModalClose } from './Modal';
import './Modal.css';
import './QrModal.css';

/**
 * QR code for opening the dashboard on a phone.
 *
 * The URL now carries the API token, which changed this component's job: it is
 * handing out a credential, not just an address. Hence the warning, and hence
 * the URL is wrapped and masked rather than shown in a single-line box — the
 * old ellipsised row showed "ht" and a Copy button, which told the user nothing
 * about what they were about to send.
 */
export function QrModal({ onClose }: { onClose: () => void }) {
  const [qrDataUrl, setQrDataUrl] = useState<string>('');
  const [url, setUrl] = useState<string>('');
  const [copied, setCopied] = useState(false);
  const [revealed, setRevealed] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      let target: string;
      try {
        const info = await fetchLocalIp();
        target = info?.url || window.location.origin;
      } catch {
        target = window.location.origin;
      }
      if (cancelled) return;
      setUrl(target);

      try {
        const dataUrl = await QRCode.toDataURL(target, {
          width: 280,
          margin: 2,
          // A scannable code needs contrast regardless of theme.
          color: { dark: '#000000', light: '#ffffff' },
        });
        if (!cancelled) setQrDataUrl(dataUrl);
      } catch (e) {
        console.error('Failed to render QR code:', e);
      }
    }

    void load();
    return () => {
      cancelled = true;
    };
  }, []);

  function handleCopy() {
    navigator.clipboard.writeText(url).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }

  return h(
    Modal,
    { onClose, class: 'qr-modal' },
    h(
      'div',
      { class: 'qr-header' },
      h('h2', null, t('qr.title')),
      h(ModalClose, { onClose, label: t('common.close') }),
    ),
    h(
      'div',
      { class: 'qr-body' },
      qrDataUrl
        ? h('img', { src: qrDataUrl, alt: t('qr.title'), class: 'qr-image' })
        : h('div', { class: 'qr-loading' }, t('app.loading')),

      h('p', { class: 'qr-hint' }, t('qr.hint')),

      h(
        'div',
        { class: 'qr-url-block' },
        h(
          'div',
          { class: 'qr-url-head' },
          h('span', { class: 'qr-url-label' }, t('qr.url')),
          h(
            'button',
            { class: 'qr-reveal-btn', type: 'button', onClick: () => setRevealed(!revealed) },
            revealed ? t('qr.hide') : t('qr.reveal'),
          ),
        ),
        h('code', { class: 'qr-url' }, revealed ? url : maskToken(url)),
        h(
          'button',
          { class: 'qr-copy-btn', type: 'button', onClick: handleCopy },
          copied ? t('qr.copied') : t('qr.copy'),
        ),
      ),

      h('p', { class: 'qr-warning' }, t('qr.tokenWarning')),
    ),
  );
}

/** Hide the token in the displayed URL, so it is not readable over a shoulder. */
function maskToken(url: string): string {
  return url.replace(/([?&]token=)[^&]+/, '$1••••••••');
}

export function QrButton() {
  const [show, setShow] = useState(false);

  return h(
    'div',
    null,
    h(
      'button',
      {
        class: 'fab-util',
        type: 'button',
        onClick: () => setShow(true),
        title: t('qr.title'),
      },
      'QR',
    ),
    show && h(QrModal, { onClose: () => setShow(false) }),
  );
}
