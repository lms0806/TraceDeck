import type { ButtonHTMLAttributes, ReactNode } from "react";

type PrimaryButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  children: ReactNode;
};

export function PrimaryButton({
  children,
  className = "",
  type = "button",
  ...props
}: PrimaryButtonProps) {
  return (
    <button
      {...props}
      type={type}
      className={`inline-flex items-center gap-2 rounded-md border-0 bg-[#28785e] px-3 py-2 font-semibold text-[#effff8] hover:bg-[#359372] disabled:cursor-default disabled:opacity-50 ${className}`}
    >
      {children}
    </button>
  );
}
