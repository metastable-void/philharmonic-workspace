import { type JSX, type MouseEvent, type ReactNode, useEffect, useRef } from "react";

export interface ModalProps {
  ariaLabel: string;
  onClose: () => void;
  children: ReactNode;
  closeOnBackdropClick?: boolean;
  closeOnEscape?: boolean;
}

let modalLockCount = 0;
let previousBodyOverflow = "";

export default function Modal({
  ariaLabel,
  onClose,
  children,
  closeOnBackdropClick = true,
  closeOnEscape = true,
}: ModalProps): JSX.Element {
  const panelRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent): void {
      if (closeOnEscape && event.key === "Escape") {
        onClose();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [closeOnEscape, onClose]);

  useEffect(() => {
    const previousActiveElement = document.activeElement;
    if (modalLockCount === 0) {
      previousBodyOverflow = document.body.style.overflow;
      document.body.style.overflow = "hidden";
    }
    modalLockCount += 1;
    panelRef.current?.focus();

    return () => {
      modalLockCount = Math.max(0, modalLockCount - 1);
      if (modalLockCount === 0) {
        document.body.style.overflow = previousBodyOverflow;
      }
      if (previousActiveElement instanceof HTMLElement) {
        previousActiveElement.focus();
      }
    };
  }, []);

  function handleBackdropClick(event: MouseEvent<HTMLDivElement>): void {
    if (closeOnBackdropClick && event.target === event.currentTarget) {
      onClose();
    }
  }

  return (
    <div className="modal-backdrop" role="presentation" onClick={handleBackdropClick}>
      <section
        className="modal-panel"
        role="dialog"
        aria-modal="true"
        aria-label={ariaLabel}
        tabIndex={-1}
        ref={panelRef}
      >
        {children}
      </section>
    </div>
  );
}
