import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import QRCode from 'qrcode';
import { fetchLocalIp } from '../lib/api';

export function QrModal({ onClose }: { onClose: () => void }) {
  const [qrDataUrl, setQrDataUrl] = useState<string>('');
  const [url, setUrl] = useState<string>('');
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    fetchLocalIp().then((info: { ip: string; port: number; url: string }) => {
      const networkUrl = info.url;
      setUrl(networkUrl);

      QRCode.toDataURL(networkUrl, {
        width: 280,
        margin: 2,
        color: { dark: '#00d4ff', light: '#1a1a1a' }
      }).then(setQrDataUrl).catch(console.error);
    }).catch(() => {
      const fallback = window.location.origin;
      setUrl(fallback);
      QRCode.toDataURL(fallback, {
        width: 280,
        margin: 2,
        color: { dark: '#00d4ff', light: '#1a1a1a' }
      }).then(setQrDataUrl).catch(console.error);
    });
  }, []);

  const handleCopy = () => {
    navigator.clipboard.writeText(url).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  return h('div', { class: 'qr-overlay', onClick: onClose },
    h('div', { class: 'qr-modal', onClick: (e: Event) => e.stopPropagation() },
      h('div', { class: 'qr-header' },
        h('h2', null, 'Scan to Connect'),
        h('button', { class: 'qr-close', onClick: onClose }, '\u00D7')
      ),
      h('div', { class: 'qr-body' },
        qrDataUrl
          ? h('img', { src: qrDataUrl, alt: 'QR Code', class: 'qr-image' })
          : h('div', { class: 'qr-loading' }, 'Generating...'),
        h('div', { class: 'qr-url-row' },
          h('span', { class: 'qr-url' }, url),
          h('button', { class: 'qr-copy-btn', onClick: handleCopy },
            copied ? 'Copied!' : 'Copy'
          )
        ),
        h('p', { class: 'qr-hint' }, 'Open this URL on your phone to access the dashboard')
      )
    )
  );
}

export function QrButton() {
  const [show, setShow] = useState(false);

  return h('div', { class: 'qr-btn-wrap' },
    h('button', {
      class: 'nav-qr-btn',
      onClick: () => setShow(true),
      title: 'Show QR code for mobile access'
    }, 'QR'),
    show && h(QrModal, { onClose: () => setShow(false) })
  );
}
