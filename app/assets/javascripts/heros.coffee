activate_buttons = ->
  $( '.update_hero' ).click ->

    code = $(this).attr( 'code' )
    action = $(this).attr( 'action' )

    html_part = $( "##{code}" )
    val = parseInt( html_part.html() )

    if action == 'plus'
      val += 1
    else

    html_part.html( val.toString() )

    adventure_id = $( '#adventure_id' ).val()
    console.log( adventure_id )


    $.ajax
      url: "/adventures/#{adventure_id}/heros"
      type: 'PUT'
      data: { code: code, action_code: action }

$(document).on('turbolinks:load', activate_buttons )