import type { ButtonHTMLAttributes, ReactNode } from "react";

export function IconButton({
  children,
  className = "",
  type = "button",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { children: ReactNode }) {
  return (
    <button
      {...props}
      type={type}
      className={`inline-flex items-center justify-center rounded border-0 bg-transparent p-1.5 text-[#91a6b6] hover:bg-[#283949] hover:text-white ${className}`}
    >
      {children}
    </button>
  );
}
