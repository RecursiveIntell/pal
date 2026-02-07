import { DndContext, PointerSensor, closestCenter, useSensor, useSensors } from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { createColumnHelper, flexRender, getCoreRowModel, useReactTable } from "@tanstack/react-table";
import type { NftRule } from "../../types/nftables";

interface Props {
  rules: NftRule[];
  selectedHandle?: number;
  onSelect: (handle: number) => void;
  onReorder: (from: number, to: number) => void;
}

const columnHelper = createColumnHelper<NftRule>();

const columns = [
  columnHelper.accessor("handle", { header: "Handle" }),
  columnHelper.accessor((r) => r.summary.description, { id: "summary", header: "Summary" }),
  columnHelper.accessor((r) => r.summary.action.type, { id: "action", header: "Action" }),
  columnHelper.accessor((r) => r.comment ?? "", { id: "comment", header: "Comment" }),
];

function SortableRow({ rule, selected, onSelect }: { rule: NftRule; selected: boolean; onSelect: () => void }) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({ id: rule.handle });
  const style = { transform: CSS.Transform.toString(transform), transition };
  const draggableProps = rule.readOnly ? {} : { ...attributes, ...listeners };

  return (
    <tr
      ref={setNodeRef}
      style={style}
      {...draggableProps}
      onClick={onSelect}
      className={`border-b border-slate-800 text-sm ${
        rule.readOnly ? "cursor-default" : "cursor-move"
      } ${selected ? "bg-slate-800/80" : "hover:bg-slate-800/50"}`}
    >
      <td className="px-3 py-2 font-mono text-xs">{rule.handle}</td>
      <td className="px-3 py-2">{rule.summary.description}</td>
      <td className="px-3 py-2">
        <span className="rounded bg-slate-700/70 px-2 py-1 text-xs">{rule.summary.action.type}</span>
        {rule.managedByService && (
          <span className="ml-2 rounded bg-blue-600/30 px-2 py-1 text-xs text-blue-200">
            Managed by {rule.managedByService}
          </span>
        )}
      </td>
      <td className="px-3 py-2 text-slate-400">{rule.comment ?? ""}</td>
    </tr>
  );
}

export function RuleTable({ rules, selectedHandle, onSelect, onReorder }: Props) {
  const table = useReactTable({ data: rules, columns, getCoreRowModel: getCoreRowModel() });
  const sensors = useSensors(useSensor(PointerSensor));

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={(event) => {
        const active = Number(event.active.id);
        const over = Number(event.over?.id);
        if (!over || active === over) {
          return;
        }
        const from = rules.findIndex((r) => r.handle === active);
        const to = rules.findIndex((r) => r.handle === over);
        if (from >= 0 && rules[from]?.readOnly) {
          return;
        }
        if (to >= 0 && rules[to]?.readOnly) {
          return;
        }
        if (from >= 0 && to >= 0) {
          const moved = arrayMove(rules, from, to);
          void moved;
          onReorder(from, to);
        }
      }}
    >
      <SortableContext items={rules.map((r) => r.handle)} strategy={verticalListSortingStrategy}>
        <table className="w-full">
          <thead className="border-b border-slate-700 bg-slate-900/70 text-left text-xs text-slate-300">
            {table.getHeaderGroups().map((group) => (
              <tr key={group.id}>
                {group.headers.map((header) => (
                  <th key={header.id} className="px-3 py-2">
                    {flexRender(header.column.columnDef.header, header.getContext())}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {rules.map((rule) => (
              <SortableRow
                key={rule.handle}
                rule={rule}
                selected={selectedHandle === rule.handle}
                onSelect={() => onSelect(rule.handle)}
              />
            ))}
          </tbody>
        </table>
      </SortableContext>
    </DndContext>
  );
}
