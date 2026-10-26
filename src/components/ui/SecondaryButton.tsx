import type { ButtonHTMLAttributes, ReactNode } from "react";

type SecondaryButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  children: ReactNode;
};

export function SecondaryButton({
  children,
  className = "",
  type = "button",
  ...props
}: SecondaryButtonProps) {
  return (
    <button
      {...props}
      type={type}
      className={`rounded border border-[#3d5264] bg-[#202e3a] px-[9px] py-1.5 text-[11px] text-[#c5d4df] hover:bg-[#2b3c4a] disabled:cursor-default disabled:opacity-50 ${className}`}
    >
      {children}
    </button>
  );
}
