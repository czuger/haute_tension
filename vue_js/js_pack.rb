require 'pp'

libs = %w( jquery.min bootstrap.min axios.min underscore.min droll.min vue )
# libs = %w( jquery bootstrap vue axios underscore )

libs_paths = []
available_paths = Dir.glob( '**/*.js' )
available_paths.reject!{ |path| File.directory?( path ) }

# p available_paths

libs.each do |lib|
  candidate_paths = []

  available_paths.each do |path|
    if path =~ /#{lib}/
      candidate_paths << path
    end
  end

  best_candidate = candidate_paths.map{ |e| [ e.length, e ] }.sort.first[1]

  puts "For lib #{lib} found #{best_candidate}"

  libs_paths << best_candidate
end

## Popper.js is ....
libs_paths.insert( 1, 'node_modules/popper.js/dist/umd/popper.min.js' )

# `cat #{libs_paths.join( ' ' )} > js/packed_libs.js`

`echo "" > js/packed_libs.js`

libs_paths.each do |path|
  `cat #{path} >> js/packed_libs.js`
  `echo "" >> js/packed_libs.js`
end

`cp node_modules/axios/dist/axios.min.map js/`
`cp node_modules/underscore/underscore-min.js.map js/`