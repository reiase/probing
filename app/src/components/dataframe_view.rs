use leptos::ev::Event;
use leptos::prelude::*;
use leptos_chartistry::*;
use thaw::*;

use probing_proto::types::DataFrame;
use probing_proto::types::Ele;
use web_sys::wasm_bindgen::JsCast;
use web_sys::MouseEvent;

#[component]
pub fn DataFrameView(df: DataFrame) -> impl IntoView {
    let head = view! {
        <TableHeader>
            <TableRow>
                <For
                    each=move || df.names.clone().into_iter()
                    key=|name| name.clone()
                    children=move |name| {
                        view! { <TableHeaderCell>{name}</TableHeaderCell> }
                    }
                />
            </TableRow>
        </TableHeader>
    };
    let nrows = df.cols.clone().iter().map(|x| x.len()).max().unwrap_or(0);
    let rows = (0..nrows)
        .map(|i| {
            let row = df
                .cols
                .iter()
                .map(move |col| match col.get(i) {
                    Ele::Nil => view! { <TableCell>{"nil".to_string()}</TableCell> },
                    Ele::I32(x) => view! { <TableCell>{x.to_string()}</TableCell> },
                    Ele::I64(x) => view! { <TableCell>{x.to_string()}</TableCell> },
                    Ele::F32(x) => view! { <TableCell>{x.to_string()}</TableCell> },
                    Ele::F64(x) => view! { <TableCell>{x.to_string()}</TableCell> },
                    Ele::Text(x) => view! { <TableCell>{x.to_string()}</TableCell> },
                    Ele::Url(x) => view! {
                        <TableCell>
                            <Link href=x.to_string()>{x.to_string()}</Link>
                        </TableCell>
                    },
                    Ele::DataTime(x) => view! { <TableCell>{x.to_string()}</TableCell> },
                })
                .collect::<Vec<_>>();
            view! { <TableRow>{row}</TableRow> }
        })
        .collect::<Vec<_>>();
    view! { <Table>{head} <TableBody>{rows}</TableBody></Table> }
}

#[derive(Clone, PartialEq)]
struct ChartDataPoint {
    x_value: f64,
    y_values: Vec<(String, f64)>, // (series_name, value)
}

#[component]
fn ChartAxisSelector(
    available_columns: ReadSignal<Vec<String>>,
    x_column: RwSignal<String>,
    y_columns: RwSignal<Vec<String>>,
) -> impl IntoView {
    let toggle_y_column = move |ev: Event| {
        let target: web_sys::EventTarget = event_target(&ev);
        let checkbox = target.dyn_ref::<web_sys::HtmlInputElement>();
        
        if let Some(checkbox) = checkbox {
            let column = checkbox.value();
            let checked = checkbox.checked();
            
            y_columns.update(|cols| {
                if checked {
                    if !cols.contains(&column) {
                        cols.push(column);
                    }
                } else {
                    cols.retain(|c| c != &column);
                }
            });
        }
    };

    view! {
        <Card>
            <CardHeader>
                <Body1>"选择图表轴"</Body1>
            </CardHeader>
            <div style="display: flex; flex-wrap: wrap; gap: 24px;">
                <div style="min-width: 200px;">
                    <h4>"X轴列"</h4>
                    <Select value=x_column>
                        <For
                            each=move || available_columns.get().into_iter()
                            key=|col| col.clone()
                            children=move |col| {
                                view! { <option value=col.clone()>{col.clone()}</option> }
                            }
                        />
                    </Select>
                </div>

                <div style="flex-grow: 1;">
                    <h4>"Y轴列 (可多选)"</h4>
                    <For
                        each=move || available_columns.get().into_iter()
                        key=|col| col.clone()
                        children=move |col| {
                            view! {
                                <Checkbox
                                    label=col.clone()
                                    value=col.clone()
                                    on:change=toggle_y_column
                                />
                            }
                        }
                    />
                </div>
            </div>
        </Card>
    }
}

#[component]
fn ChartFilterManager(
    available_columns: ReadSignal<Vec<String>>,
    applied_filters: RwSignal<Vec<(String, String, String)>>,
) -> impl IntoView {
    let filter_column = RwSignal::new(String::new());
    let filter_operator = RwSignal::new("equals".to_string());
    let filter_value = RwSignal::new(String::new());
    
    let add_filter = move |_: MouseEvent| {
        let col = filter_column.get();
        let op = filter_operator.get();
        let val = filter_value.get();
        
        if !col.is_empty() && !val.is_empty() {
            applied_filters.update(|filters| {
                filters.push((col.clone(), op.clone(), val.clone()));
            });
            filter_value.set("".to_string());
        }
    };
    
    let remove_filter = move |key: String| {
        applied_filters.update(|filters| {
            filters.retain(|(col, op, val)| {
                format!("{}-{}-{}", col, op, val) != key
            });
        });
    };

    view! {
        <Card>
            <CardHeader>
                <Body1>"过滤条件"</Body1>
            </CardHeader>
            <div>
                <p>"列"</p>
                <Select default_value="" value=filter_column>
                    <For
                        each=move || available_columns.get().into_iter()
                        key=|col| col.clone()
                        children=move |col| {
                            view! { <option value=col.clone()>{col.clone()}</option> }
                        }
                    />
                </Select>
            </div>

            <div>
                <p>"操作符"</p>
                <Select default_value="equals" value=filter_operator>
                    <option value="equals">"等于"</option>
                    <option value="contains">"包含"</option>
                    <option value="greater">"大于"</option>
                    <option value="less">"小于"</option>
                </Select>
            </div>

            <div>
                <p>"值"</p>
                <Input
                    placeholder="输入过滤值"
                    value=filter_value
                    on_blur=move |ev| filter_value.set(event_target_value(&ev))
                />
            </div>

            <Button appearance=ButtonAppearance::Primary on_click=add_filter>
                "添加过滤器"
            </Button>

            <div style="margin-top: 16px;">
                <h4>"已应用的过滤器:"</h4>
                <For
                    each=move || applied_filters.get()
                    key=|(col, op, val)| format!("{}-{}-{}", col, op, val)
                    children=move |(col, op, val)| {
                        let filter_text = match op.as_str() {
                            "equals" => format!("{}等于{}", col, val),
                            "contains" => format!("{}包含{}", col, val),
                            "greater" => format!("{}大于{}", col, val),
                            "less" => format!("{}小于{}", col, val),
                            _ => format!("{} {} {}", col, op, val),
                        };
                        view! {
                            <Tag
                                dismissible=true
                                on_dismiss=move |_| remove_filter(
                                    format!("{}-{}-{}", col, op, val),
                                )
                            >
                                {filter_text}
                            </Tag>
                        }
                    }
                />
            </div>
        </Card>
    }
}

#[component]
fn ChartRenderer(
    #[prop(into)]
    chart_data: Signal<Vec<ChartDataPoint>>,
    selected_y_columns: ReadSignal<Vec<String>>,
) -> impl IntoView {
    let chart_view = move || {
        let data = chart_data.get();
        if data.is_empty() {
            return view! { <div class="no-data">"请选择有效的X轴和Y轴列"</div> }.into_any();
        }
        
        let y_cols = selected_y_columns.get();
        
        // 创建系列
        let series_builder = Series::new(|point: &ChartDataPoint| point.x_value);
        
        // 为每个Y列添加一条线
        let series_with_lines = y_cols.iter().fold(
            series_builder,
            |series, col_name| {
                let col_name_clone = col_name.clone();
                series.line(
                    Line::new(move |point: &ChartDataPoint| {
                        point.y_values.iter()
                            .find(|(name, _)| name == &col_name_clone)
                            .map(|(_, value)| *value)
                            .unwrap_or(0.0)
                    })
                    .with_name(col_name.clone())
                )
            }
        );
        
        view! {
            <Chart
                aspect_ratio=AspectRatio::from_outer_ratio(800.0, 400.0)
                top=RotatedLabel::middle("数据可视化")
                left=TickLabels::aligned_floats()
                right=Legend::end()
                bottom=TickLabels::aligned_floats()
                inner=[
                    AxisMarker::left_edge().into_inner(),
                    AxisMarker::bottom_edge().into_inner(),
                    XGridLine::default().into_inner(),
                    YGridLine::default().into_inner(),
                    XGuideLine::over_data().into_inner(),
                    YGuideLine::over_mouse().into_inner(),
                ]
                tooltip=Tooltip::left_cursor()
                series=series_with_lines
                data=Signal::derive(move || data.clone())
            />
        }.into_any()
    };

    view! {
        <Card>
            <CardHeader>
                <Body1>"图表"</Body1>
            </CardHeader>
            {chart_view}
        </Card>
    }
}

#[component]
pub fn DataFrameChartView(df: DataFrame) -> impl IntoView {
    // 用户选择
    let x_column = RwSignal::new(String::new());
    let y_columns = RwSignal::new(Vec::<String>::new());
    let available_columns = RwSignal::new(df.names.clone());
    
    // 过滤设置
    let applied_filters = RwSignal::new(Vec::<(String, String, String)>::new());

    // 从DataFrame中提取数据点
    let chart_data = move || {
        let x_col = x_column.get();
        let y_cols = y_columns.get();
        let filters = applied_filters.get();
        
        if x_col.is_empty() || y_cols.is_empty() {
            return Vec::new();
        }
        
        // 找出列的索引位置
        let x_idx = df.names.iter().position(|name| name == &x_col);
        let y_indices: Vec<(usize, String)> = y_cols
            .iter()
            .filter_map(|col| {
                df.names.iter().position(|name| name == col)
                    .map(|idx| (idx, col.clone()))
            })
            .collect();
        
        if x_idx.is_none() || y_indices.is_empty() {
            return Vec::new();
        }
        
        let x_idx = x_idx.unwrap();
        
        // 构建数据点
        let mut chart_points = Vec::new();
        
        for row_idx in 0..df.len() {
            // 应用过滤器
            let mut include_row = true;
            
            for (filter_col, filter_op, filter_val) in &filters {
                if let Some(col_idx) = df.names.iter().position(|name| name == filter_col) {
                    let cell_value = df.cols[col_idx].get(row_idx);
                    let cell_str = match &cell_value {
                        Ele::Text(s) => s.clone(),
                        Ele::I32(i) => i.to_string(),
                        Ele::I64(i) => i.to_string(),
                        Ele::F32(f) => f.to_string(),
                        Ele::F64(f) => f.to_string(),
                        _ => "".to_string(),
                    };
                    
                    match filter_op.as_str() {
                        "equals" => include_row = cell_str == *filter_val,
                        "contains" => include_row = cell_str.contains(filter_val),
                        "greater" => {
                            if let Ok(cell_num) = cell_str.parse::<f64>() {
                                if let Ok(filter_num) = filter_val.parse::<f64>() {
                                    include_row = cell_num > filter_num;
                                }
                            }
                        },
                        "less" => {
                            if let Ok(cell_num) = cell_str.parse::<f64>() {
                                if let Ok(filter_num) = filter_val.parse::<f64>() {
                                    include_row = cell_num < filter_num;
                                }
                            }
                        },
                        _ => {}
                    }
                    
                    if !include_row {
                        break;
                    }
                }
            }
            
            if !include_row {
                continue;
            }
            
            // 获取X值
            let x_ele = df.cols[x_idx].get(row_idx);
            let x_value = match x_ele {
                Ele::I32(i) => i as f64,
                Ele::I64(i) => i as f64,
                Ele::F32(f) => f as f64,
                Ele::F64(f) => f,
                _ => continue, // 跳过不能转换为数值的X值
            };
            
            // 获取所有Y值
            let mut y_values = Vec::new();
            for (y_idx, y_name) in &y_indices {
                let y_ele = df.cols[*y_idx].get(row_idx);
                match y_ele {
                    Ele::I32(i) => y_values.push((y_name.clone(), i as f64)),
                    Ele::I64(i) => y_values.push((y_name.clone(), i as f64)),
                    Ele::F32(f) => y_values.push((y_name.clone(), f as f64)),
                    Ele::F64(f) => y_values.push((y_name.clone(), f)),
                    _ => {} // 跳过不能转换为数值的Y值
                }
            }
            
            chart_points.push(ChartDataPoint {
                x_value,
                y_values,
            });
        }
        
        chart_points
    };

    view! {
        <div
            class="chart-builder"
            style="width: 100%; display: flex; flex-direction: column; gap: 16px;"
        >
            <ChartAxisSelector 
                available_columns=available_columns.read_only() 
                x_column 
                y_columns
            />
            
            <ChartFilterManager 
                available_columns=available_columns.read_only() 
                applied_filters
            />
            
            <ChartRenderer 
                chart_data=Signal::derive(move || chart_data())
                selected_y_columns=y_columns.read_only()
            />
        </div>
    }
}