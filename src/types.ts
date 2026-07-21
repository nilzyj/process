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
  page?: number;
  page_size?: number;
}

export interface PaginatedResult {
  records: MediaRecord[];
  total: number;
}

export interface Stats {
  total: number;
  by_type: { media_type: string; count: number }[];
  by_status: { status: string; count: number }[];
  by_year: { year: number; count: number }[];
  by_country: { country: string; count: number }[];
  by_tags: { tag: string; count: number }[];
  progress_buckets: { label: string; count: number }[];
  type_status: { media_type: string; status: string; count: number }[];
  completion_rates: { media_type: string; completed: number; total: number }[];
  daily_activity: { date: string; count: number }[];
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
  '动漫': { icon: '📺', label: '动漫', color: '#F48FB1' },
  '漫画': { icon: '📖', label: '漫画', color: '#81D4FA' },
  '电影': { icon: '🎬', label: '电影', color: '#CE93D8' },
  '电视剧': { icon: '📺', label: '电视剧', color: '#FFAB91' },
  '书籍': { icon: '📚', label: '书籍', color: '#A5D6A7' },
  '纪录片': { icon: '🎥', label: '纪录片', color: '#FFD54F' },
  'TED': { icon: '🎤', label: 'TED', color: '#90CAF9' },
  '播客': { icon: '🎙️', label: '播客', color: '#F48FB1' },
};

export const STATUS_STYLES: Record<string, { label: string; color: string; bg: string }> = {
  '待办': { label: '待办', color: '#BCAAA4', bg: 'rgba(188,170,164,0.12)' },
  '进行中': { label: '进行中', color: '#81D4FA', bg: 'rgba(129,212,250,0.12)' },
  '已完成': { label: '已完成', color: '#A5D6A7', bg: 'rgba(165,214,167,0.12)' },
};

export const TYPE_LIST = ['全部', '动漫', '漫画', '电影', '电视剧', '书籍', '纪录片', 'TED', '播客'];
export const STATUS_LIST = ['全部', '待办', '进行中', '已完成'];
