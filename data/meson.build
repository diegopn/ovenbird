install_data('io.github.diegopn.ovenbird.desktop', install_dir: get_option('datadir') / 'applications')
install_data('io.github.diegopn.ovenbird.metainfo.xml', install_dir: get_option('datadir') / 'metainfo')
install_data('io.github.diegopn.ovenbird.svg', install_dir: get_option('datadir') / 'icons' / 'hicolor' / 'scalable' / 'apps')
install_data('io.github.diegopn.ovenbird-symbolic.svg', install_dir: get_option('datadir') / 'icons' / 'hicolor' / 'symbolic' / 'apps')
foreach icon_size : ['32', '48', '64', '128', '256', '512']
  icon_dimensions = icon_size + 'x' + icon_size
  install_data('icons' / icon_dimensions / 'io.github.diegopn.ovenbird.png', install_dir: get_option('datadir') / 'icons' / 'hicolor' / icon_dimensions / 'apps')
endforeach
install_data('../LICENSE', install_dir: get_option('datadir') / 'licenses' / application_id)
