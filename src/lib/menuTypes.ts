export interface MenuItem {
  type: 'item';
  id: string;
  label: string;
  danger?: boolean;
  disabled?: boolean;
  onClick: (event: MouseEvent) => void;
}

export interface MenuSeparator {
  type: 'separator';
}

export type Item = MenuItem | MenuSeparator;
