export interface MediaRecord {
  id: number;
  record_name: string;
  season: number | null;
  remark: string | null;
  media_type: string | null;
  status: string | null;
  end_time: string | null;
  country: string | null;
  tags: string | null;
  current_episode: number | null;
  total_episode: number | null;
  year: number | null;
  modify_time: string | null;
}

export interface NewRecord {
  record_name: string;
  season: number | null;
  remark: string | null;
  media_type: string | null;
  status: string | null;
  end_time: string | null;
  country: string | null;
  tags: string | null;
  current_episode: number | null;
  total_episode: number | null;
  year: number | null;
}

export interface UpdateRecord extends NewRecord {
  id: number;
}

export interface RecordFilter {
  search?: string;
  media_type?: string;
  status?: string;
  tag?: string;
  page?: number;
  page_size?: number;
}

export interface PaginatedResult {
  records: MediaRecord[];
  total: number;
}

export interface SeriesStat {
  tag: string;
  total: number;
  completed: number;
  by_type: { media_type: string; count: number }[];
}

export interface Stats {
  total: number;
  by_type: { media_type: string; count: number }[];
  by_status: { status: string; count: number }[];
  by_year: { year: number; count: number }[];
  by_country: { country: string; count: number }[];
  by_tags: { tag: string; count: number }[];
  series_stats: SeriesStat[];
  progress_buckets: { label: string; count: number }[];
  type_status: { media_type: string; status: string; count: number }[];
  completion_rates: { media_type: string; completed: number; total: number }[];
  daily_activity: { date: string; count: number }[];
  monthly_end: { month: string; count: number }[];
  recent: {
    new_today: number; new_week: number; new_month: number;
    completed_today: number; completed_week: number; completed_month: number;
  };
}

export interface DbConfig {
  host: string;
  port: number;
  user: string;
  password: string;
  database: string;
}

export interface TypeStyle {
  icon: string;
  label: string;
  color: string;
}

export const MEDIA_TYPES: Record<string, TypeStyle> = {
  '动漫': { icon: '📺', label: '动漫', color: '#f97316' },
  '漫画': { icon: '📖', label: '漫画', color: '#06b6d4' },
  '电影': { icon: '🎬', label: '电影', color: '#e50914' },
  '电视剧': { icon: '📺', label: '电视剧', color: '#22d3ee' },
  '书籍': { icon: '📚', label: '书籍', color: '#f59e0b' },
  '纪录片': { icon: '🎥', label: '纪录片', color: '#22c55e' },
  'TED': { icon: '🎤', label: 'TED', color: '#e2e8f0' },
  '播客': { icon: '🎙️', label: '播客', color: '#ec4899' },
};

export const STATUS_STYLES: Record<string, { label: string; color: string; bg: string }> = {
  '待办': { label: '待办', color: '#888888', bg: 'rgba(136,136,136,0.15)' },
  '进行中': { label: '进行中', color: '#22d3ee', bg: 'rgba(34,211,238,0.15)' },
  '已完成': { label: '已完成', color: '#22c55e', bg: 'rgba(34,197,94,0.15)' },
};

export const TYPE_LIST = ['全部', '动漫', '漫画', '电影', '电视剧', '书籍', '纪录片', 'TED', '播客'];
export const STATUS_LIST = ['全部', '待办', '进行中', '已完成'];
