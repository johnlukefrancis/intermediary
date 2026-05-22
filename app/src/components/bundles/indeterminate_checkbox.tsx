// Path: app/src/components/bundles/indeterminate_checkbox.tsx
// Description: Checkbox component that supports the DOM indeterminate state

import { useEffect, useRef } from "react";

interface IndeterminateCheckboxProps {
  id: string;
  checked: boolean;
  indeterminate: boolean;
  onChange: () => void;
}

export function IndeterminateCheckbox({
  id,
  checked,
  indeterminate,
  onChange,
}: IndeterminateCheckboxProps): React.JSX.Element {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (inputRef.current) {
      inputRef.current.indeterminate = indeterminate;
    }
  }, [indeterminate]);

  return (
    <label className="vintage-toggle">
      <input
        ref={inputRef}
        id={id}
        type="checkbox"
        checked={checked}
        onChange={onChange}
      />
      <span className={`vintage-toggle-track${indeterminate ? " indeterminate" : ""}`} />
    </label>
  );
}
